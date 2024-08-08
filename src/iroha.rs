use iroha_crypto::KeyPair;
use iroha_data_model::{
    account::AccountId,
    prelude::CompoundPredicate,
    query::{
        parameters::{FetchSize, IterableQueryParams, Pagination},
        predicate::{projectors, AstPredicate, HasPredicateBox, HasPrototype},
        IterableQuery, IterableQueryBox, IterableQueryOutputBatchBox, IterableQueryWithFilter,
        IterableQueryWithFilterFor, IterableQueryWithParams, QueryRequest, QueryResponse,
    },
    ValidationFail,
};
use iroha_telemetry::metrics::Status;
use parity_scale_codec::{DecodeAll as _, Encode};
use reqwest::StatusCode;
use url::Url;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to perform HTTP request to Iroha: {0}")]
    Http(#[from] reqwest::Error),
    #[error("failed to encode/decode SCALE binary data: {0}")]
    Scale(#[from] parity_scale_codec::Error),
    #[error("Iroha Query validation failed: {reason}")]
    QueryValidationFail { reason: ValidationFail },
    #[error("unexpected response status code: {0}")]
    UnexpectedResponseCode(StatusCode),
    #[error("expected iterable query response")]
    ExpectedIterableResponse,
    #[error("failed to extract query output")]
    ExtractQueryOutput,
}

#[derive(Debug, Clone)]
pub struct Client {
    authority: AccountId,
    key_pair: KeyPair,
    torii_url: Url,
    http: reqwest::Client,
}

impl Client {
    pub fn new(authority: AccountId, key_pair: KeyPair, torii_url: Url) -> Self {
        Self {
            authority,
            key_pair,
            torii_url,
            http: reqwest::Client::new(),
        }
    }

    pub fn query<Q>(
        &self,
        query: Q,
    ) -> QueryBuilder<Q, <<Q as IterableQuery>::Item as HasPredicateBox>::PredicateBoxType>
    where
        Q: IterableQuery,
    {
        QueryBuilder::new(&self, query)
    }

    pub async fn status(&self) -> Result<Status, Error> {
        todo!()
    }

    async fn execute_query_request(&self, request: QueryRequest) -> Result<QueryResponse, Error> {
        let signed = request
            .with_authority(self.authority.clone())
            .sign(&self.key_pair);

        let response = self
            .http
            .post(self.torii_url.join("/query").unwrap())
            .body(signed.encode())
            .send()
            .await?;

        let status = response.status();
        let bytes = response.bytes().await?;

        match status {
            StatusCode::OK => Ok(QueryResponse::decode_all(&mut bytes.as_ref())?),
            StatusCode::BAD_REQUEST
            | StatusCode::UNAUTHORIZED
            | StatusCode::FORBIDDEN
            | StatusCode::NOT_FOUND
            | StatusCode::UNPROCESSABLE_ENTITY => {
                let reason = ValidationFail::decode_all(&mut bytes.as_ref())?;
                Err(Error::QueryValidationFail { reason })
            }
            unknown => Err(Error::UnexpectedResponseCode(unknown)),
        }
    }
}

pub struct QueryBuilder<'e, Q, P> {
    client: &'e Client,
    query: Q,
    filter: CompoundPredicate<P>,
}

impl<'a, Q, P> QueryBuilder<'a, Q, P> {
    fn new(client: &'a Client, query: Q) -> Self {
        Self {
            client,
            query,
            filter: CompoundPredicate::PASS,
        }
    }

    #[must_use]
    pub fn filter<B, O>(mut self, predicate_builder: B) -> Self
    where
        P: HasPrototype,
        B: FnOnce(P::Prototype<projectors::BaseProjector<P>>) -> O,
        O: AstPredicate<P>,
    {
        use iroha_data_model::query::predicate::predicate_ast_extensions::AstPredicateExt as _;

        self.filter = self
            .filter
            .and(predicate_builder(Default::default()).normalize());
        self
    }

    // pub fn paginate(self, pagination: Pagination) -> Self {
    //     unimplemented!()
    // }
}

impl<Q, P> QueryBuilder<'_, Q, P>
where
    Q: IterableQuery,
    Q::Item: HasPredicateBox<PredicateBoxType = P>,
    IterableQueryBox: From<IterableQueryWithFilterFor<Q>>,
    Vec<Q::Item>: TryFrom<IterableQueryOutputBatchBox>,
    <Vec<Q::Item> as TryFrom<IterableQueryOutputBatchBox>>::Error: core::fmt::Debug,
{
    pub async fn all(self) -> Result<Vec<Q::Item>, Error> {
        let start = QueryRequest::StartIterable(IterableQueryWithParams::new(
            IterableQueryBox::from(IterableQueryWithFilter::new(self.query, self.filter)),
            IterableQueryParams::default(),
        ));

        let QueryResponse::Iterable(response) = self.client.execute_query_request(start).await?
        else {
            return Err(Error::ExpectedIterableResponse);
        };
        let (batch, mut cursor) = response.into_parts();

        let mut items: Vec<Q::Item> = batch.try_into().map_err(|_| Error::ExtractQueryOutput)?;

        while let Some(forward) = cursor {
            let next = QueryRequest::ContinueIterable(forward);
            let QueryResponse::Iterable(response) = self.client.execute_query_request(next).await?
            else {
                return Err(Error::ExpectedIterableResponse);
            };
            let (batch, new_cursor) = response.into_parts();
            let batch: Vec<_> = batch.try_into().map_err(|_| Error::ExtractQueryOutput)?;
            items.extend(batch);
            cursor = new_cursor;
        }

        Ok(items)
    }

    pub async fn one(self) -> Result<Option<Q::Item>, Error> {
        let request = QueryRequest::StartIterable(IterableQueryWithParams::new(
            IterableQueryBox::from(IterableQueryWithFilter::new(self.query, self.filter)),
            IterableQueryParams::new(
                // FIXME: how to construct pagination?
                Pagination::default(),
                Default::default(),
                FetchSize::new(Some(1.try_into().unwrap())),
            ),
        ));
        let QueryResponse::Iterable(response) = self.client.execute_query_request(request).await?
        else {
            return Err(Error::ExpectedIterableResponse);
        };
        let (batch, _cursor) = response.into_parts();
        let items: Vec<Q::Item> = batch.try_into().map_err(|_| Error::ExtractQueryOutput)?;
        let one = items.into_iter().take(1).next();
        Ok(one)
    }
}

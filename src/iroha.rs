use iroha_crypto::KeyPair;
use iroha_data_model::{
    account::AccountId,
    prelude::CompoundPredicate,
    query::{
        parameters::{Pagination, QueryParams},
        predicate::{projectors, AstPredicate, HasPredicateBox, HasPrototype},
        Query, QueryBox, QueryOutputBatchBox, QueryRequest, QueryResponse, QueryWithFilter,
        QueryWithFilterFor, QueryWithParams, SingularQuery, SingularQueryBox,
        SingularQueryOutputBox,
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
    #[error("expected singular query response")]
    ExpectedSingularResponse,
    #[error("expected to got all data in a single request, got a forward cursor")]
    UnexpectedContinuationCursor,
    #[error("failed to extract query output")]
    ExtractQueryOutput,
    #[error("expected one or zero elements in the result, got {got}")]
    ExpectedOne { got: usize },
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
    ) -> QueryBuilder<Q, <<Q as Query>::Item as HasPredicateBox>::PredicateBoxType>
    where
        Q: Query,
    {
        QueryBuilder::new(&self, query)
    }

    pub async fn query_singular<Q>(&self, query: Q) -> Result<<Q as SingularQuery>::Output, Error>
    where
        Q: SingularQuery,
        SingularQueryBox: From<Q>,
        Q::Output: TryFrom<SingularQueryOutputBox>,
        <Q::Output as TryFrom<SingularQueryOutputBox>>::Error: std::fmt::Debug,
    {
        let QueryResponse::Singular(boxed) = self
            .execute_query_request(QueryRequest::Singular(query.into()))
            .await?
        else {
            return Err(Error::ExpectedSingularResponse);
        };

        Ok(boxed
            .try_into()
            .expect("BUG: iroha data model contract failed"))
    }

    pub async fn status(&self) -> Result<Status, Error> {
        let response = self
            .http
            .get(self.torii_url.join("/status").unwrap())
            .header(reqwest::header::ACCEPT, "application/x-parity-scale")
            .send()
            .await?;

        let status = response.status();
        let bytes = response.bytes().await?;

        match status {
            StatusCode::OK => {
                let data = Status::decode_all(&mut bytes.as_ref())?;
                Ok(data)
            }
            unknown => Err(Error::UnexpectedResponseCode(unknown)),
        }
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
    pagination: Pagination,
}

impl<'a, Q, P> QueryBuilder<'a, Q, P> {
    fn new(client: &'a Client, query: Q) -> Self {
        Self {
            client,
            query,
            filter: CompoundPredicate::PASS,
            pagination: Pagination::default(),
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

    pub fn paginate(mut self, pagination: impl Into<Pagination>) -> Self {
        self.pagination = pagination.into();
        self
    }
}

impl<Q, P> QueryBuilder<'_, Q, P>
where
    Q: Query,
    Q::Item: HasPredicateBox<PredicateBoxType = P>,
    QueryBox: From<QueryWithFilterFor<Q>>,
    Vec<Q::Item>: TryFrom<QueryOutputBatchBox>,
    <Vec<Q::Item> as TryFrom<QueryOutputBatchBox>>::Error: core::fmt::Debug,
{
    pub async fn all(self) -> Result<Vec<Q::Item>, Error> {
        let start = QueryRequest::Start(QueryWithParams::new(
            QueryBox::from(QueryWithFilter::new(self.query, self.filter)),
            QueryParams::new(self.pagination, Default::default(), Default::default()),
        ));

        let QueryResponse::Iterable(response) = self.client.execute_query_request(start).await?
        else {
            return Err(Error::ExpectedIterableResponse);
        };
        let (batch, cursor) = response.into_parts();
        if cursor.is_some() {
            return Err(Error::UnexpectedContinuationCursor);
        }
        let items: Vec<Q::Item> = batch.try_into().map_err(|_| Error::ExtractQueryOutput)?;

        Ok(items)
    }

    pub async fn one(self) -> Result<Option<Q::Item>, Error> {
        let all = self.all().await?;
        if all.len() > 1 {
            return Err(Error::ExpectedOne { got: all.len() });
        }
        let mut items = all.into_iter();
        let one = items.next();
        Ok(one)
    }
}

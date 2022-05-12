use std::{fmt::Debug, sync::Arc};

use awc::{Client as ActixClient, ClientResponse as RespActix};
use color_eyre::{
    eyre::{self, eyre, Context},
    Result,
};
use iroha_client::{
    client::{Client as IrohaClient, PaginatedQueryOutput, ResponseHandler},
    http::Response as RespIroha,
};
use iroha_data_model::{
    metadata::UnlimitedMetadata,
    prelude::{Instruction, Pagination, Query, QueryBox, Value},
};
use iroha_telemetry::metrics::Status;

use request_builder::ActixReqBuilder;

mod request_builder {
    use std::{borrow::Borrow, collections::HashMap};

    use super::{eyre, Context, RespActix, RespIroha, Result};
    use awc::Client;
    use iroha_client::http::{Headers, Method, RequestBuilder};

    /// TODO avoid allocations
    pub struct ActixReqBuilder {
        url: String,
        method: Method,
        headers: Option<Headers>,
        params: Option<Vec<(String, String)>>,
        body: Option<Vec<u8>>,
    }

    impl ActixReqBuilder {
        pub async fn send(self, client: &Client) -> Result<RespIroha<Vec<u8>>> {
            let Self {
                headers,
                url,
                method,
                params,
                body,
            } = self;

            let mut req = client.request(method, url);

            if let Some(map) = headers {
                req = map
                    .into_iter()
                    .fold(req, |acc, item| acc.insert_header(item));
            }

            if let Some(params) = params {
                let map: HashMap<String, String> = params.into_iter().collect();
                req = req.query(&map)?;
            }

            let req = if let Some(body) = body {
                req.send_body(body)
            } else {
                req.send()
            };

            let resp = req
                .await
                .map_err(|x| eyre!("Failed to make HTTP request: {}", x))?;

            let resp = ResponseWrapper::new(resp)
                .into_iroha_response()
                .await
                .wrap_err("Failed to map Actix response to Iroha response")?;

            Ok(resp)
        }
    }

    impl RequestBuilder for ActixReqBuilder {
        fn new<U>(method: Method, url: U) -> Self
        where
            U: AsRef<str>,
        {
            Self {
                method,
                url: url.as_ref().to_owned(),
                headers: None,
                params: None,
                body: None,
            }
        }

        fn bytes(self, data: Vec<u8>) -> Self {
            Self {
                body: Some(data),
                ..self
            }
        }

        fn headers(self, headers: iroha_client::http::Headers) -> Self {
            Self {
                headers: Some(headers),
                ..self
            }
        }

        fn params<P, K, V>(self, params: P) -> Self
        where
            P: IntoIterator,
            P::Item: std::borrow::Borrow<(K, V)>,
            K: AsRef<str>,
            V: ToString,
        {
            Self {
                params: Some(
                    params
                        .into_iter()
                        .map(|pair| {
                            let (k, v) = pair.borrow();
                            (k.as_ref().to_owned(), v.to_string())
                        })
                        .collect(),
                ),
                ..self
            }
        }
    }

    pub struct ResponseWrapper<T>(RespActix<T>);

    impl<T> ResponseWrapper<T>
    where
        T: actix::prelude::Stream<Item = Result<actix_web::web::Bytes, awc::error::PayloadError>>,
    {
        pub fn new(resp: RespActix<T>) -> Self {
            Self(resp)
        }

        pub async fn into_iroha_response(self) -> Result<RespIroha<Vec<u8>>> {
            let Self(mut resp) = self;

            let mut builder = RespIroha::builder().status(resp.status());

            for (k, v) in resp.headers().iter() {
                builder = builder.header(k, v);
            }

            let bytes: Vec<u8> = resp
                .body()
                .await
                .wrap_err("Failed to consume response body")?
                .to_vec();

            builder.body(bytes).wrap_err("Failed to build response")
        }
    }
}

pub struct IrohaClientWrap {
    iroha: Arc<IrohaClient>,
    http: ActixClient,
}

impl IrohaClientWrap {
    pub fn new(iroha_client: Arc<IrohaClient>) -> Self {
        Self {
            iroha: iroha_client,
            http: ActixClient::default(),
        }
    }

    pub async fn request_with_pagination<R>(
        &self,
        request: R,
        pagination: Pagination,
    ) -> Result<PaginatedQueryOutput<R>>
    where
        R: Query + Into<QueryBox> + Debug,
        <R::Output as TryFrom<Value>>::Error: Into<eyre::Error>,
    {
        let (req, resp_handler): (ActixReqBuilder, _) =
            self.iroha.prepare_query_request(request, pagination)?;
        // FIXME response should be a trait!
        let resp = req.send(&self.http).await?;
        resp_handler.handle(resp)
    }

    pub async fn request<R>(&self, request: R) -> Result<R::Output>
    where
        R: Query + Into<QueryBox> + Debug,
        <R::Output as TryFrom<Value>>::Error: Into<eyre::Error>,
    {
        self.request_with_pagination(request, Pagination::new(None, None))
            .await
            .map(PaginatedQueryOutput::only_output)
    }

    pub async fn get_status(&self) -> Result<Status> {
        let (req, resp_handler) = self.iroha.prepare_status_request::<ActixReqBuilder>();
        let resp = req.send(&self.http).await?;
        resp_handler.handle(resp)
    }

    pub async fn submit(&self, instruction: impl Into<Instruction> + Debug) -> Result<()> {
        let (req, _, resp_handler) = self
            .iroha
            .prepare_transaction_request::<ActixReqBuilder>(
                self.iroha
                    .build_transaction(
                        (vec![instruction.into()]).into_iter().into(),
                        UnlimitedMetadata::new(),
                    )
                    .wrap_err("Failed to build transaction")?,
            )
            .wrap_err("Failed to prepare transaction request")?;

        let resp = req.send(&self.http).await?;
        let () = resp_handler.handle(resp)?;

        Ok(())
    }
}

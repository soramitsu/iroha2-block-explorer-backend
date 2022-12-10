use std::{fmt::Debug, sync::Arc};

use awc::{Client as ActixClient, ClientResponse as RespActix};
use color_eyre::{
    eyre::{self, eyre, Context},
    Result,
};
use iroha_client::{
    client::{Client as IrohaClient, ClientQueryError, ClientQueryOutput, ResponseHandler},
    http::Response as RespIroha,
};
use iroha_data_model::prelude::Sorting;
use iroha_data_model::{
    metadata::UnlimitedMetadata,
    predicate::PredicateBox,
    prelude::{Instruction, Pagination, Query, QueryBox, Value},
};
use iroha_telemetry::metrics::Status;

use request_builder::ActixReqBuilder;

mod request_builder {
    use std::{collections::HashMap, str::FromStr};

    use super::{eyre, Context, RespActix, RespIroha, Result};
    use awc::{Client, ClientRequest};
    use http::header::{HeaderMap, HeaderName};
    use iroha_client::http::{Method, RequestBuilder};

    trait HeaderMapConsumerExt {
        fn set_headers(self, headers: HeaderMap) -> Result<Self>
        where
            Self: Sized;
    }

    impl HeaderMapConsumerExt for ClientRequest {
        fn set_headers(mut self, headers: HeaderMap) -> Result<Self> {
            let mut prev_header_name: Option<HeaderName> = None;

            for (name, value) in headers {
                let name = name.or(prev_header_name).ok_or_else(|| {
                    eyre!("At least one header name should be emitted by `HeaderMap`")
                })?;
                self = self.insert_header((&name, value));
                prev_header_name = Some(name);
            }

            Ok(self)
        }
    }

    /// TODO avoid allocations
    pub struct ActixReqBuilder {
        // req: ClientRequest,
        url: String,
        method: Method,
        headers: Result<HeaderMap>,
        params: Vec<(String, String)>,
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

            req = req
                .set_headers(headers.wrap_err("Headers construction failed")?)
                .wrap_err("Headers setting failed")?;

            req = {
                let map: HashMap<String, String> = params.into_iter().collect();
                req.query(&map)
                    .wrap_err("Failed to set query params to the request")?
            };

            let req = if let Some(body) = body {
                req.send_body(body)
            } else {
                req.send()
            };

            let resp = req
                .await
                .map_err(|x| eyre!("Failed to make HTTP request: {:?}", x))?;

            let resp = ResponseWrapper::new(resp)
                .into_iroha_response()
                .await
                .wrap_err("Failed to map Actix response to Iroha response")?;

            Ok(resp)
        }
    }

    impl RequestBuilder for ActixReqBuilder {
        fn new(method: Method, url: impl AsRef<str>) -> Self {
            Self {
                method,
                url: url.as_ref().to_owned(),
                headers: Ok(HeaderMap::new()),
                params: Vec::new(),
                body: None,
            }
        }

        fn param<K, V>(mut self, key: K, value: V) -> Self
        where
            K: AsRef<str>,
            V: ToString,
        {
            self.params
                .push((key.as_ref().to_string(), value.to_string()));
            self
        }

        fn header<N, V>(self, name: N, value: V) -> Self
        where
            N: AsRef<str>,
            V: ToString,
        {
            Self {
                headers: self.headers.and_then(|mut map| {
                    let name: http::header::HeaderName =
                        FromStr::from_str(name.as_ref()).wrap_err("Failed to parse header name")?;
                    let value = value
                        .to_string()
                        .parse()
                        .wrap_err("Failed to parse header value")?;
                    map.insert(name, value);
                    Ok(map)
                }),
                ..self
            }
        }

        fn body(self, data: Vec<u8>) -> Self {
            Self {
                body: Some(data),
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

pub struct QueryBuilder<R>
where
    R: Query,
{
    request: R,
    pagination: Option<Pagination>,
    filter: Option<PredicateBox>,
}

impl<R> QueryBuilder<R>
where
    R: Query,
{
    pub fn new(request: R) -> Self {
        Self {
            request,
            pagination: None,
            filter: None,
        }
    }

    pub fn with_pagination(self, value: Pagination) -> Self {
        Self {
            pagination: Some(value),
            ..self
        }
    }

    // unused for now
    pub fn _with_filter(self, value: PredicateBox) -> Self {
        Self {
            filter: Some(value),
            ..self
        }
    }
}

impl IrohaClientWrap {
    pub fn new(iroha_client: Arc<IrohaClient>) -> Self {
        Self {
            iroha: iroha_client,
            http: ActixClient::default(),
        }
    }

    pub async fn request<R>(
        &self,
        query: QueryBuilder<R>,
    ) -> Result<ClientQueryOutput<R>, ClientQueryError>
    where
        R: Query + Into<QueryBox> + Debug,
        <R::Output as TryFrom<Value>>::Error: Into<eyre::Error>,
    {
        let (req, resp_handler): (ActixReqBuilder, _) = self
            .iroha
            .prepare_query_request(
                query.request,
                query.pagination.unwrap_or_default(),
                Sorting::default(),
                query.filter.unwrap_or_default(),
            )
            .wrap_err("Failed to prepare query request")?;
        // FIXME response should be a trait!
        let resp = req
            .send(&self.http)
            .await
            .wrap_err("Failed to make query")?;
        resp_handler.handle(resp)
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
        resp_handler.handle(resp)?;

        Ok(())
    }
}

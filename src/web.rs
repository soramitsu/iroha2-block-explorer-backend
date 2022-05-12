use std::fmt::Debug;
use std::sync::Arc;

use actix_web::{
    error::ResponseError, get, http, web, App, HttpResponse, HttpServer, Responder, Scope,
};
use derive_more::Display;
use serde::Serialize;

use crate::iroha_client_wrap::IrohaClientWrap;
use pagination::{Paginated, PaginationQueryParams};

/// Web app state that may be injected in runtime
pub struct AppData {
    /// Pre-initialized Iroha Client
    iroha_client: IrohaClientWrap,
}

impl AppData {
    /// Creates new state with provided client
    pub fn new(client: IrohaClientWrap) -> Self {
        Self {
            iroha_client: client,
        }
    }
}

/// General error for all endpoints
#[derive(Display, Debug)]
enum WebError {
    /// Some error that should be logged, but shouldn't be returned to
    /// a client. Server should return an empty 500 error instead.
    Internal(color_eyre::Report),
}

impl ResponseError for WebError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(http::header::ContentType::html())
            .body(match self {
                // We don't want to expose internal errors to the client, so here it is omitted.
                // `actix-web` will log it anyway.
                WebError::Internal(_) => "Internal Server Error",
            })
    }

    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::INTERNAL_SERVER_ERROR
    }
}

impl From<color_eyre::Report> for WebError {
    fn from(err: color_eyre::Report) -> Self {
        Self::Internal(err)
    }
}

impl From<iroha_data_model::ParseError> for WebError {
    fn from(err: iroha_data_model::ParseError) -> Self {
        Self::Internal(err.into())
    }
}

mod pagination;

mod accounts {
    use super::{assets::AssetDTO, *};
    use iroha_data_model::prelude::{
        Account, AccountId, FindAccountById, FindAllAccounts, Metadata,
    };
    use serde::de::{self, Deserialize, Deserializer, Visitor};
    use std::{fmt, str::FromStr};

    #[derive(Serialize)]
    pub struct AccountDTO {
        id: String,
        assets: Vec<AssetDTO>,
        metadata: Metadata,
        roles: Vec<String>,
    }

    impl From<Account> for AccountDTO {
        fn from(account: Account) -> Self {
            let assets: Vec<AssetDTO> = account
                .assets()
                .into_iter()
                .map(|asset|
                    // FIXME clone
                    AssetDTO::from(asset.clone()
                  ))
                .collect();

            let roles: Vec<String> = account.roles().into_iter().map(|x| x.to_string()).collect();

            Self {
                id: account.id().to_string(),
                assets,
                metadata:
                // FIXME clone
                account.metadata().clone(),
                roles,
            }
        }
    }

    pub struct AccountIdInPath(pub AccountId);

    impl<'de> Deserialize<'de> for AccountIdInPath {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct AccountIdInPathVisitor;

            impl<'de> Visitor<'de> for AccountIdInPathVisitor {
                type Value = AccountIdInPath;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(formatter, "a string in a format `alice@wonderland`")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    AccountId::from_str(v)
                        .map(AccountIdInPath)
                        .map_err(|_parse_error| E::invalid_value(de::Unexpected::Str(v), &self))
                }
            }

            deserializer.deserialize_string(AccountIdInPathVisitor)
        }
    }

    #[get("/{id}")]
    async fn show(
        data: web::Data<AppData>,
        id: web::Path<AccountIdInPath>,
    ) -> Result<impl Responder, WebError> {
        // TODO handle not found error
        let account = data
            .iroha_client
            .request(FindAccountById::new(id.into_inner().0))
            .await?;
        Ok(web::Json(AccountDTO::from(account)))
    }

    #[get("")]
    async fn index(
        data: web::Data<AppData>,
        web::Query(pagination): web::Query<PaginationQueryParams>,
    ) -> Result<impl Responder, WebError> {
        let paginated: Paginated<_> = data
            .iroha_client
            .request_with_pagination(FindAllAccounts::new(), pagination.into())
            .await?
            .try_into()?;

        Ok(web::Json(paginated))
    }

    pub fn service() -> Scope {
        web::scope("/accounts").service(index).service(show)
    }
}

mod domains {
    use super::{accounts::AccountDTO, asset_definitions::AssetDefinitionDTO, *};
    use iroha_data_model::prelude::{Domain, DomainId, FindAllDomains, FindDomainById, Metadata};

    #[derive(Serialize)]
    struct DomainDTO {
        id: String,
        accounts: Vec<AccountDTO>,
        logo: Option<String>,
        metadata: Metadata,
        asset_definitions: Vec<AssetDefinitionDTO>,
        // TODO amount of triggers
        triggers: u32,
    }

    impl From<Domain> for DomainDTO {
        fn from(domain: Domain) -> Self {
            Self {
                id: domain.id().to_string(),
                accounts: domain
                    .accounts()
                    .into_iter()
                    .map(|acc|
                        // FIXME clone
                        AccountDTO::from(acc.clone()))
                    .collect(),
                logo: domain.logo().as_ref().map(|x| x.as_ref().to_owned()),
                metadata: domain.metadata().clone(), // FIXME clone
                asset_definitions: AssetDefinitionDTO::vec_from_map(
                    domain
                        // FIXME clone
                        .asset_definitions()
                        .cloned(),
                ),
                triggers: 0,
            }
        }
    }

    #[get("/{id}")]
    async fn show(
        data: web::Data<AppData>,
        path: web::Path<String>,
    ) -> Result<impl Responder, WebError> {
        let domain_id: DomainId = path.into_inner().parse()?;
        // TODO handle not found error
        let domain = data
            .iroha_client
            .request(FindDomainById::new(domain_id))
            .await?;
        Ok(web::Json(DomainDTO::from(domain)))
    }

    #[get("")]
    async fn index(
        data: web::Data<AppData>,
        pagination: web::Query<PaginationQueryParams>,
    ) -> Result<impl Responder, WebError> {
        let paginated: Paginated<_> = data
            .iroha_client
            .request_with_pagination(FindAllDomains::new(), pagination.into_inner().into())
            .await?
            .try_into()?;
        let paginated: Paginated<Vec<DomainDTO>> =
            paginated.map(|domains| domains.into_iter().map(|x| x.into()).collect());
        Ok(web::Json(paginated))
    }

    pub fn service() -> Scope {
        web::scope("/domains").service(index).service(show)
    }
}

mod assets {
    use super::{accounts::AccountIdInPath, asset_definitions::AssetDefinitionIdInPath, *};
    use iroha_data_model::prelude::{
        Asset, AssetId, AssetValue, AssetValueType, FindAllAssets, FindAssetById, Metadata,
    };
    use serde::Deserialize;

    #[derive(Serialize)]
    #[serde(tag = "t", content = "c")]
    pub enum AssetValueDTO {
        Quantity(u32),
        BigQuantity(u128),
        Fixed(String),
        Store(Metadata),
    }

    impl From<AssetValue> for AssetValueDTO {
        fn from(val: AssetValue) -> Self {
            use AssetValue::*;

            match val {
                Quantity(x) => Self::Quantity(x),
                BigQuantity(x) => Self::BigQuantity(x),
                Fixed(x) => Self::Fixed(f64::from(x).to_string()),
                Store(x) => Self::Store(x),
            }
        }
    }

    #[derive(Serialize)]
    pub struct AssetDTO {
        account_id: String,
        definition_id: String,
        value: AssetValueDTO,
    }

    impl From<Asset> for AssetDTO {
        fn from(val: Asset) -> Self {
            let id = val.id();
            // FIXME clone
            let value = val.value().clone();

            Self {
                account_id: id.account_id.to_string(),
                definition_id: id.definition_id.to_string(),
                value: AssetValueDTO::from(value),
            }
        }
    }

    #[derive(Serialize)]
    pub struct AssetValueTypeDTO(AssetValueType);

    #[derive(Deserialize)]
    pub struct AssetIdInPath {
        pub account_id: AccountIdInPath,
        pub definition_id: AssetDefinitionIdInPath,
    }

    impl From<AssetIdInPath> for AssetId {
        fn from(val: AssetIdInPath) -> Self {
            AssetId::new(val.definition_id.0, val.account_id.0)
        }
    }

    #[get("")]
    async fn index(
        data: web::Data<AppData>,
        pagination: web::Query<PaginationQueryParams>,
    ) -> Result<impl Responder, WebError> {
        let data: Paginated<_> = data
            .iroha_client
            .request_with_pagination(FindAllAssets::new(), pagination.into_inner().into())
            .await?
            .try_into()?;
        let data: Paginated<Vec<AssetDTO>> =
            data.map(|assets| assets.into_iter().map(|x| x.into()).collect());
        Ok(web::Json(data))
    }

    #[get("/{definition_id}/{account_id}")]
    async fn show(
        data: web::Data<AppData>,
        path: web::Path<AssetIdInPath>,
    ) -> Result<impl Responder, WebError> {
        let asset_id: AssetId = path.into_inner().into();
        // TODO handle not found error
        let asset = data
            .iroha_client
            .request(FindAssetById::new(asset_id))
            .await?;
        Ok(web::Json(AssetDTO::from(asset)))
    }

    pub fn service() -> Scope {
        web::scope("/assets").service(index).service(show)
    }
}

mod asset_definitions {
    use super::*;
    use iroha_data_model::{
        asset::Mintable,
        prelude::{
            AssetDefinition, AssetDefinitionEntry, AssetDefinitionId, AssetValueType,
            FindAllAssetsDefinitions,
        },
    };
    use serde::de::{self, Deserialize, Deserializer, Visitor};
    use std::{fmt, str::FromStr};

    #[derive(Serialize)]
    pub struct AssetDefinitionDTO {
        id: String,
        value_type: AssetValueTypeDTO,
        mintable: Mintable,
    }

    impl AssetDefinitionDTO {
        pub fn vec_from_map<T>(map: T) -> Vec<Self>
        where
            T: ExactSizeIterator + Iterator<Item = AssetDefinitionEntry>,
        {
            map.into_iter()
                .map(|def| def.definition().clone().into())
                .collect()
        }
    }

    impl From<AssetDefinition> for AssetDefinitionDTO {
        fn from(definition: AssetDefinition) -> Self {
            Self {
                id: definition.id().to_string(),
                value_type: AssetValueTypeDTO(*definition.value_type()),
                mintable: *definition.mintable(),
            }
        }
    }

    pub struct AssetDefinitionIdInPath(pub AssetDefinitionId);

    impl<'de> Deserialize<'de> for AssetDefinitionIdInPath {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct AssetDefinitionIdInPathVisitor;

            impl<'de> Visitor<'de> for AssetDefinitionIdInPathVisitor {
                type Value = AssetDefinitionIdInPath;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(formatter, "a string in a format `rose#wonderland`")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    AssetDefinitionId::from_str(v)
                        .map(AssetDefinitionIdInPath)
                        .map_err(|_parse_error| E::invalid_value(de::Unexpected::Str(v), &self))
                }
            }

            deserializer.deserialize_string(AssetDefinitionIdInPathVisitor)
        }
    }

    #[derive(Serialize)]
    pub struct AssetValueTypeDTO(AssetValueType);

    // WIP iroha does not support FindAssetDefinitionById yet
    // https://github.com/hyperledger/iroha/pull/2126
    // #[get("/{id}")]
    // async fn show(
    //     data: web::Data<AppState>,
    //     id: web::Path<AssetDefinitionIdInPath>,
    // ) -> Result<impl Responder, WebError> {
    //     let assets = data.with_client(|client| client.request(FindAssetDefinitionKeyValueByIdAndKey::new()))??;
    //     let assets: Vec<AssetDTO> = assets.into_iter().map(|x| x.into()).collect();
    //     Ok(web::Json(assets))
    // }

    #[get("")]
    async fn index(
        data: web::Data<AppData>,
        pagination: web::Query<PaginationQueryParams>,
    ) -> Result<impl Responder, WebError> {
        let data: Paginated<_> = data
            .iroha_client
            .request_with_pagination(FindAllAssetsDefinitions::new(), pagination.0.into())
            .await?
            .try_into()?;
        let data = data.map::<Vec<AssetDefinitionDTO>, _>(|items| {
            items.into_iter().map(|x| x.into()).collect()
        });
        Ok(web::Json(data))
    }

    pub fn service() -> Scope {
        web::scope("/asset-definitions").service(index)
        // .service(show)
    }
}

mod peer {
    use super::*;
    use iroha_data_model::prelude::{FindAllPeers, Peer, PeerId};

    #[derive(Serialize)]
    pub struct PeerDTO(PeerId);

    impl From<Peer> for PeerDTO {
        fn from(val: Peer) -> Self {
            Self(val.id)
        }
    }

    #[get("/peers")]
    async fn peers(
        data: web::Data<AppData>,
        pagination: web::Query<PaginationQueryParams>,
    ) -> Result<impl Responder, WebError> {
        let data: Paginated<_> = data
            .iroha_client
            .request_with_pagination(FindAllPeers::new(), pagination.0.into())
            .await?
            .try_into()?;
        let data =
            data.map::<Vec<PeerDTO>, _>(|items| items.into_iter().map(|x| x.into()).collect());
        Ok(web::Json(data))
    }

    #[get("/status")]
    async fn status(data: web::Data<AppData>) -> Result<impl Responder, WebError> {
        let status = data.iroha_client.get_status().await?;
        Ok(web::Json(status))
    }

    pub fn service() -> Scope {
        web::scope("/peer").service(peers).service(status)
    }
}

mod roles {
    use super::*;
    use iroha_data_model::prelude::{FindAllRoles, Role};

    #[derive(Serialize)]
    pub struct RoleDTO(Role);

    impl From<Role> for RoleDTO {
        fn from(val: Role) -> Self {
            Self(val)
        }
    }

    #[get("")]
    async fn index(
        app: web::Data<AppData>,
        pagination: web::Query<PaginationQueryParams>,
    ) -> Result<impl Responder, WebError> {
        let data: Paginated<_> = app
            .iroha_client
            // TODO add an issue about absense of `FindAllRoles::new()`?
            .request_with_pagination(FindAllRoles {}, pagination.0.into())
            .await?
            .try_into()?;
        let data =
            data.map::<Vec<RoleDTO>, _>(|items| items.into_iter().map(|x| x.into()).collect());
        Ok(web::Json(data))
    }

    pub fn service() -> Scope {
        web::scope("/roles").service(index)
    }
}

async fn default_route() -> impl Responder {
    HttpResponse::NotFound().body("Not Found")
}

#[get("/")]
async fn root_health_check() -> impl Responder {
    HttpResponse::Ok().body("Welcome to Iroha 2 Block Explorer!")
}

pub struct ServerInitData {
    iroha_client: Arc<iroha_client::client::Client>,
}

impl ServerInitData {
    pub fn new(iroha_client: Arc<iroha_client::client::Client>) -> Self {
        Self { iroha_client }
    }
}

/// Initializes a server listening on `127.0.0.1:<port>`. It should be awaited to be actually started.
pub fn server(
    ServerInitData { iroha_client }: ServerInitData,
    port: u16,
) -> color_eyre::Result<actix_server::Server> {
    let server = HttpServer::new(move || {
        let client_wrap = crate::iroha_client_wrap::IrohaClientWrap::new(iroha_client.clone());
        let app_data = web::Data::new(AppData::new(client_wrap));

        App::new()
            .app_data(app_data)
            .wrap(super::logger::TracingLogger::default())
            .service(
                web::scope("/api/v1")
                    .service(root_health_check)
                    .service(accounts::service())
                    .service(domains::service())
                    .service(assets::service())
                    .service(asset_definitions::service())
                    .service(roles::service())
                    .service(peer::service()),
            )
            .default_service(web::route().to(default_route))
    })
    .bind(("127.0.0.1", port))?
    .run();

    Ok(server)
}

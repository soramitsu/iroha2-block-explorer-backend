use actix_web::{
    error::ResponseError, get, http, web, App, HttpResponse, HttpServer, Responder, Scope,
};
use color_eyre::eyre::{eyre, WrapErr};
use derive_more::Display;
use iroha_client::client::Client as IrohaClient;
use serde::Serialize;
use std::sync::{Mutex, MutexGuard};

/// Web app state that may be injected in runtime
pub struct AppState {
    /// Pre-initialized Iroha Client
    iroha_client: Mutex<IrohaClient>,
}

impl AppState {
    /// Tries to lock the client's mutex
    ///
    /// # Errors
    /// Fails if mutex lock fails
    pub fn lock_client(&self) -> color_eyre::Result<MutexGuard<IrohaClient>> {
        Ok(self
            .iroha_client
            .lock()
            .map_err(|_| eyre!("failed to lock iroha client mutex"))?)
    }

    /// Locks client mutex and passes it into the closure. Returns the closure output.
    ///
    /// # Errors
    /// Fails if mutex lock fails
    pub fn with_client<F, T>(&self, op: F) -> color_eyre::Result<T>
    where
        F: FnOnce(&mut MutexGuard<IrohaClient>) -> T,
        T: Sized,
    {
        let mut client = self.lock_client()?;
        let res = op(&mut client);
        Ok(res)
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

mod pagination {
    use super::*;

    // pub const DEFAULT_PAGE_SIZE: u32 = 15;

    /// Represents some items list with its pagination data
    #[derive(Serialize)]
    pub struct Paginated<T> {
        pagination: Pagination,
        items: Vec<T>,
    }

    impl<T> Paginated<T> {
        /// Wraps some items list with a provided pagination data
        pub fn wrap(items: Vec<T>, pagination: Pagination) -> Self {
            Self { items, pagination }
        }

        /// It is primarily to fake real pagination
        pub fn from_the_whole_list(items: Vec<T>) -> color_eyre::Result<Self> {
            let len: u32 = items.len().try_into()?;
            let new_self = Self::wrap(items, Pagination::new(1, len, 1));
            Ok(new_self)
        }
    }

    /// Represents pagination data
    #[derive(Serialize)]
    pub struct Pagination {
        /// Current page. Starts from 1
        page_number: u32,
        /// Represents pagination scale
        page_size: u32,
        /// Total count of data pages in according to [`Pagination::page_size`]
        pages: u32,
    }

    impl Pagination {
        pub fn new(page_number: u32, page_size: u32, pages: u32) -> Self {
            Self {
                page_number,
                page_size,
                pages,
            }
        }
    }
}

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
        fn from(
            Account {
                assets,
                id,
                metadata,
                roles,
                ..
            }: Account,
        ) -> Self {
            let assets: Vec<AssetDTO> = assets
                .into_iter()
                .map(|(_, asset)| AssetDTO::from(asset))
                .collect();

            let roles: Vec<String> = roles.into_iter().map(|x| x.to_string()).collect();

            Self {
                id: id.to_string(),
                assets,
                metadata,
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
                        .map(|x| AccountIdInPath(x))
                        .map_err(|_parse_error| {
                            E::invalid_value(de::Unexpected::Str(&v), &self)
                        })
                }
            }

            deserializer.deserialize_string(AccountIdInPathVisitor)
        }
    }

    #[get("/{id}")]
    async fn show(
        data: web::Data<AppState>,
        id: web::Path<AccountIdInPath>,
    ) -> Result<impl Responder, WebError> {
        // TODO handle not found error
        let account = data
            .with_client(|client| client.request(FindAccountById::new(id.into_inner().0)))??;
        Ok(web::Json(AccountDTO::from(account)))
    }

    #[get("")]
    async fn index(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
        let accounts: Vec<Account> =
            data.with_client(|client| client.request(FindAllAccounts::new()))??;

        let accounts: Vec<AccountDTO> =
            accounts.into_iter().map(|x| AccountDTO::from(x)).collect();

        let paginated = pagination::Paginated::from_the_whole_list(accounts)?;
        Ok(web::Json(paginated))
    }

    pub fn service() -> Scope {
        web::scope("/accounts").service(index).service(show)
    }
}

mod domains {
    use super::{accounts::AccountDTO, asset_definitions::AssetDefinitionDTO, *};
    use iroha_data_model::prelude::{
        Domain, DomainId, FindAllDomains, FindDomainById, Metadata,
    };

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
        fn from(
            Domain {
                id,
                accounts,
                logo,
                metadata,
                asset_definitions,
            }: Domain,
        ) -> Self {
            Self {
                id: id.to_string(),
                accounts: accounts
                    .into_iter()
                    .map(|(_, acc)| AccountDTO::from(acc))
                    .collect(),
                logo: logo.map(|x| x.as_ref().to_owned()),
                metadata,
                asset_definitions: AssetDefinitionDTO::vec_from_map(asset_definitions),
                triggers: 0,
            }
        }
    }

    #[get("/{id}")]
    async fn show(
        data: web::Data<AppState>,
        path: web::Path<String>,
    ) -> Result<impl Responder, WebError> {
        let domain_id_raw = path.into_inner();
        let domain_id = DomainId::new(&domain_id_raw).wrap_err("")?;
        // TODO handle not found error
        let domain = data.with_client(|client| {
            let query = FindDomainById::new(domain_id);
            client.request(query)
        })??;
        Ok(web::Json(DomainDTO::from(domain)))
    }

    #[get("")]
    async fn index(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
        let domains = data.with_client(|client| client.request(FindAllDomains::new()))??;
        let domains: Vec<DomainDTO> = domains.into_iter().map(|x| x.into()).collect();
        Ok(web::Json(pagination::Paginated::from_the_whole_list(
            domains,
        )?))
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
            Self {
                account_id: val.id.account_id.to_string(),
                definition_id: val.id.definition_id.to_string(),
                value: AssetValueDTO::from(val.value),
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

    impl Into<AssetId> for AssetIdInPath {
        fn into(self) -> AssetId {
            AssetId::new(self.definition_id.0, self.account_id.0)
        }
    }

    #[get("")]
    async fn index(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
        let assets = data.with_client(|client| client.request(FindAllAssets::new()))??;
        let assets: Vec<AssetDTO> = assets.into_iter().map(|x| x.into()).collect();
        let paginated = pagination::Paginated::from_the_whole_list(assets)?;
        Ok(web::Json(paginated))
    }

    #[get("/{definition_id}/{account_id}")]
    async fn show(
        data: web::Data<AppState>,
        path: web::Path<AssetIdInPath>,
    ) -> Result<impl Responder, WebError> {
        let asset_id: AssetId = path.into_inner().into();
        // TODO handle not found error
        let asset =
            data.with_client(|client| client.request(FindAssetById::new(asset_id)))??;
        Ok(web::Json(AssetDTO::from(asset)))
    }

    pub fn service() -> Scope {
        web::scope("/assets").service(index).service(show)
    }
}

mod asset_definitions {
    use super::*;
    use iroha_data_model::{
        asset::AssetDefinitionsMap,
        prelude::{
            AssetDefinition, AssetDefinitionId, AssetValueType, FindAllAssetsDefinitions,
        },
    };
    use serde::de::{self, Deserialize, Deserializer, Visitor};
    use std::{fmt, str::FromStr};

    #[derive(Serialize)]
    pub struct AssetDefinitionDTO {
        id: String,
        value_type: AssetValueTypeDTO,
        mintable: bool,
    }

    impl AssetDefinitionDTO {
        pub fn vec_from_map(map: AssetDefinitionsMap) -> Vec<Self> {
            map.into_iter()
                .map(|(_, def)| def.definition.into())
                .collect()
        }
    }

    impl From<AssetDefinition> for AssetDefinitionDTO {
        fn from(
            AssetDefinition {
                value_type,
                id,
                mintable,
                ..
            }: AssetDefinition,
        ) -> Self {
            Self {
                id: id.to_string(),
                value_type: AssetValueTypeDTO(value_type),
                mintable,
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
                        .map(|x| AssetDefinitionIdInPath(x))
                        .map_err(|_parse_error| {
                            E::invalid_value(de::Unexpected::Str(&v), &self)
                        })
                }
            }

            deserializer.deserialize_string(AssetDefinitionIdInPathVisitor)
        }
    }

    #[derive(Serialize)]
    pub struct AssetValueTypeDTO(AssetValueType);

    // WIP iroha does not support FindAssetDefinitionById yet
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
    async fn index(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
        let items =
            data.with_client(|client| client.request(FindAllAssetsDefinitions::new()))??;
        let items: Vec<AssetDefinitionDTO> = items.into_iter().map(|x| x.into()).collect();
        Ok(web::Json(pagination::Paginated::from_the_whole_list(
            items,
        )?))
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
    async fn peers(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
        let items = data.with_client(|client| client.request(FindAllPeers::new()))??;
        let items: Vec<PeerDTO> = items.into_iter().map(|x| x.into()).collect();
        Ok(web::Json(items))
    }

    #[get("/status")]
    async fn status(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
        let status =
            data.with_client(|client| client.get_status().wrap_err("failed to get status"))??;

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
    async fn index(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
        let items = data.with_client(|client| client.request(FindAllRoles {}))??;
        let items: Vec<RoleDTO> = items.into_iter().map(|x| x.into()).collect();
        Ok(web::Json(items))
    }

    pub fn service() -> Scope {
        web::scope("/roles").service(index)
    }
}

impl AppState {
    pub fn new(client_config: &iroha_client::Configuration) -> Self {
        Self {
            iroha_client: Mutex::new(IrohaClient::new(client_config)),
        }
    }
}

async fn default_route() -> impl Responder {
    HttpResponse::NotFound().body("Not Found")
}

#[get("/")]
async fn root_health_check() -> impl Responder {
    HttpResponse::Ok().body("Welcome to Iroha 2 Block Explorer!")
}

/// Initializes a server listening on `127.0.0.1:<port>`. It should be awaited to be actually started.
pub fn server(state: AppState, port: u16) -> color_eyre::Result<actix_server::Server> {
    let state = web::Data::new(state);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(super::logger::TracingLogger::default())
            .service(root_health_check)
            .service(accounts::service())
            .service(domains::service())
            .service(assets::service())
            .service(asset_definitions::service())
            .service(roles::service())
            .service(peer::service())
            .default_service(web::route().to(default_route))
    })
    .bind(("127.0.0.1", port))?
    .run();

    Ok(server)
}

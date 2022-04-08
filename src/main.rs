mod logger {
    use tracing::{subscriber::set_global_default, Subscriber};
    pub use tracing_actix_web::TracingLogger;
    use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
    use tracing_log::LogTracer;
    use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

    pub use tracing::info;

    /// Compose multiple layers into a `tracing`'s subscriber.
    fn get_subscriber(name: String, env_filter: String) -> impl Subscriber + Send + Sync {
        let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new(env_filter));
        let bunyan_formatter = BunyanFormattingLayer::new(name.into(), std::io::stdout);
        Registry::default()
            .with(env_filter)
            .with(JsonStorageLayer)
            .with(bunyan_formatter)
    }

    /// Register a subscriber as global default to process span data.
    ///
    /// It should only be called once!
    fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
        LogTracer::init().expect("Failed to set logger");
        set_global_default(subscriber).expect("Failed to set subscriber");
    }

    pub fn setup() {
        let subscriber = get_subscriber("iroha2-explorer-web".into(), "info".into());
        init_subscriber(subscriber);
    }
}

mod args {
    use color_eyre::Help as _;
    use color_eyre::{eyre::Context as _, Report, Result};
    use iroha_client::Configuration as IrohaClientConfiguration;
    use std::str::FromStr;
    use structopt::StructOpt;

    #[derive(Debug, StructOpt)]
    #[structopt(about = "Iroha 2 Explorer Backend")]
    pub struct Args {
        #[structopt(short, long, default_value = "4000", env)]
        pub port: u16,

        #[structopt(
            short = "c",
            long,
            default_value = "client_config.json",
            help = "`iroha_client` JSON configuration path"
        )]
        pub client_config: String,
    }

    #[derive(Debug)]
    pub struct ArgsClientConfig(pub IrohaClientConfiguration);

    impl ArgsClientConfig {
        pub fn new(args: &Args) -> Result<Self> {
            Self::from_str(&args.client_config)
        }
    }

    impl FromStr for ArgsClientConfig {
        type Err = Report;

        fn from_str(file: &str) -> Result<Self> {
            use std::fs::File;

            let file_opened: File = File::open(file)
                .wrap_err_with(|| format!("failed to open client config file: {}", file))
                .with_suggestion(|| {
                    "try to specify another file with `-c` or `--client-config` argument"
                })?;
            let cfg: IrohaClientConfiguration = serde_json::from_reader(file_opened)
                .wrap_err_with(|| format!("failed to parse client config file: {}", file))?;
            Ok(Self(cfg))
        }
    }
}

mod web {
    use actix_web::{
        error::ResponseError, get, http, web, App, HttpResponse, HttpServer, Responder, Result,
        Scope,
    };
    use color_eyre::eyre::{eyre, WrapErr};
    use derive_more::Display;
    use iroha_client::client::Client as IrohaClient;
    use serde::Serialize;
    use std::sync::{Mutex, MutexGuard};

    pub struct AppState {
        iroha_client: Mutex<IrohaClient>,
    }

    impl AppState {
        pub fn lock_client(&self) -> color_eyre::Result<MutexGuard<IrohaClient>> {
            Ok(self
                .iroha_client
                .lock()
                .map_err(|_| eyre!("failed to lock iroha client mutex"))?)
        }

        pub fn with_client<F, T>(&self, op: F) -> color_eyre::Result<T>
        where
            F: Fn(&mut MutexGuard<IrohaClient>) -> T,
            T: Sized,
        {
            let mut client = self.lock_client()?;
            let res = op(&mut client);
            Ok(res)
        }
    }

    #[derive(Display, Debug)]
    enum WebError {
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

    mod accounts {
        use super::{assets::AssetDTO, *};
        use iroha_crypto::PublicKey;
        use iroha_data_model::prelude::{
            Account, FindAllAccounts, Metadata, PermissionToken, SignatureCheckCondition,
        };
        use serde::Serialize;

        #[derive(Serialize)]
        pub struct AccountDTO {
            id: String,
            assets: Vec<AssetDTO>,
            signatories: Vec<PublicKey>,
            signature_check_condition: SignatureCheckCondition,
            metadata: Metadata,
            permission_tokens: Vec<PermissionToken>,
            roles: Vec<String>,
        }

        impl From<Account> for AccountDTO {
            fn from(
                Account {
                    assets,
                    id,
                    metadata,
                    roles,
                    signatories,
                    permission_tokens,
                    signature_check_condition,
                    ..
                }: Account,
            ) -> Self {
                let assets: Vec<AssetDTO> = assets
                    .into_iter()
                    .map(|(_, asset)| AssetDTO::from(asset))
                    .collect();

                let roles: Vec<String> = roles.into_iter().map(|x| x.to_string()).collect();
                let signatories: Vec<PublicKey> = signatories.into_iter().collect();
                let permission_tokens: Vec<PermissionToken> =
                    permission_tokens.into_iter().collect();

                Self {
                    id: id.to_string(),
                    assets,
                    metadata,
                    roles,
                    signatories,
                    permission_tokens,
                    signature_check_condition,
                }
            }
        }

        #[get("")]
        async fn index(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
            let accounts: Vec<Account> =
                data.with_client(|client| client.request(FindAllAccounts::new()))??;

            let accounts: Vec<AccountDTO> =
                accounts.into_iter().map(|x| AccountDTO::from(x)).collect();

            Ok(web::Json(accounts))
        }

        pub fn service() -> Scope {
            web::scope("/accounts").service(index)
        }
    }

    mod domains {
        use iroha_data_model::prelude::{Domain, FindAllDomains, Metadata};
        use serde::Serialize;

        use super::{accounts::AccountDTO, assets::AssetDefinitionDTO, *};

        #[derive(Serialize)]
        struct DomainDTO {
            id: String,
            accounts: Vec<AccountDTO>,
            logo: Option<String>,
            metadata: Metadata, // add metadata and asset definitions here?
            asset_definitions: Vec<AssetDefinitionDTO>,
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
                }
            }
        }

        #[get("")]
        async fn index(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
            let domains = data.with_client(|client| client.request(FindAllDomains::new()))??;
            let domains: Vec<DomainDTO> = domains.into_iter().map(|x| x.into()).collect();
            Ok(web::Json(domains))
        }

        pub fn service() -> Scope {
            web::scope("/domains").service(index)
        }
    }

    mod assets {
        use super::*;
        use iroha_data_model::{
            asset::AssetDefinitionsMap,
            prelude::{
                Asset, AssetDefinition, AssetValue, AssetValueType, FindAllAssets,
                FindAllAssetsDefinitions, Metadata,
            },
        };
        use serde::Serialize;

        #[derive(Serialize)]
        #[serde(tag = "t", content = "c")]
        pub enum AssetValueDTO {
            Quantity(u32),
            BigQuantity(String),
            Fixed(String),
            Store(Metadata), // no associated data?
        }

        impl From<AssetValue> for AssetValueDTO {
            fn from(val: AssetValue) -> Self {
                use AssetValue::*;

                match val {
                    Quantity(x) => Self::Quantity(x),
                    BigQuantity(x) => Self::BigQuantity(x.to_string()),
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

        #[derive(Serialize)]
        pub struct AssetValueTypeDTO(AssetValueType);

        #[get("")]
        async fn index(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
            let assets = data.with_client(|client| client.request(FindAllAssets::new()))??;
            let assets: Vec<AssetDTO> = assets.into_iter().map(|x| x.into()).collect();
            Ok(web::Json(assets))
        }

        #[get("/definitions")]
        async fn definitions_index(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
            let items =
                data.with_client(|client| client.request(FindAllAssetsDefinitions::new()))??;
            let items: Vec<AssetDefinitionDTO> = items.into_iter().map(|x| x.into()).collect();
            Ok(web::Json(items))
        }

        pub fn service() -> Scope {
            web::scope("/assets")
                .service(index)
                .service(definitions_index)
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

        // FindAllParameters is WIP for now
        // https://github.com/hyperledger/iroha/issues/1966
        // #[get("/parameters")]
        // async fn parameters(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
        //     let items = data.with_client(|client| client.request(FindAllParameters::new()))??;
        //     let items: Vec<PeerDTO> = items.into_iter().map(|x| x.into()).collect();
        //     Ok(web::Json(items))
        // }

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

    pub fn server(state: AppState, port: u16) -> color_eyre::Result<actix_server::Server> {
        let state = web::Data::new(state);

        let server = HttpServer::new(move || {
            App::new()
                .app_data(state.clone())
                .wrap(super::logger::TracingLogger::default())
                .service(accounts::service())
                .service(domains::service())
                .service(assets::service())
                .service(roles::service())
                .service(peer::service())
                .default_service(web::route().to(default_route))
        })
        .bind(("127.0.0.1", port))?
        .run();

        Ok(server)
    }
}

use color_eyre::{eyre::WrapErr, Result};

#[actix_web::main]
async fn main() -> Result<()> {
    let args: args::Args = structopt::StructOpt::from_args();
    let client_config = args::ArgsClientConfig::new(&args)?;

    logger::setup();
    logger::info!("Server going to listen on {}", args.port);

    web::server(web::AppState::new(&client_config.0), args.port)?
        .await
        .wrap_err("Server run failed")
}

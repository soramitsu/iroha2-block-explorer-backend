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
        error::ResponseError, get, http, web, App, HttpResponse, HttpServer, Responder, Scope,
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

    mod status {
        use super::*;

        #[get("")]
        async fn index(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
            let status =
                data.with_client(|client| client.get_status().wrap_err("failed to get status"))??;

            Ok(web::Json(status))
        }

        pub fn service() -> Scope {
            web::scope("/status").service(index)
        }
    }

    mod accounts {
        use super::{assets::AssetDTO, *};
        use iroha_data_model::prelude::{Account, FindAllAccounts};
        use serde::Serialize;

        #[derive(Serialize)]
        pub struct AccountDTO {
            id: String,
            assets: Vec<AssetDTO>,
            // what else add here?
        }

        impl From<Account> for AccountDTO {
            fn from(acc: Account) -> Self {
                let assets: Vec<AssetDTO> = acc
                    .assets
                    .into_iter()
                    .map(|(_, asset)| AssetDTO::from(asset))
                    .collect();

                Self {
                    id: acc.id.to_string(),
                    assets,
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
        use iroha_data_model::prelude::{Domain, FindAllDomains};
        use serde::Serialize;

        use super::{accounts::AccountDTO, *};

        #[derive(Serialize)]
        struct DomainDTO {
            id: String,
            accounts: Vec<AccountDTO>,
        }

        impl From<Domain> for DomainDTO {
            fn from(domain: Domain) -> Self {
                Self {
                    id: domain.id.to_string(),
                    accounts: domain
                        .accounts
                        .into_iter()
                        .map(|(_, acc)| AccountDTO::from(acc))
                        .collect(),
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
        use iroha_data_model::prelude::{
            Asset, AssetDefinition, AssetValue, AssetValueType, FindAllAssets,
            FindAllAssetsDefinitions,
        };
        use serde::Serialize;

        #[derive(Serialize)]
        #[serde(tag = "t", content = "c")]
        pub enum AssetValueDTO {
            Quantity(u32),
            BigQuantity(String),
            Fixed(String),
            Store, // no associated data?
        }

        impl From<AssetValue> for AssetValueDTO {
            fn from(val: AssetValue) -> Self {
                use AssetValue::*;

                match val {
                    Quantity(x) => Self::Quantity(x),
                    BigQuantity(x) => Self::BigQuantity(x.to_string()),
                    Fixed(x) => Self::Fixed(f64::from(x).to_string()),
                    Store(_) => Self::Store,
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
        }

        impl From<AssetDefinition> for AssetDefinitionDTO {
            fn from(val: AssetDefinition) -> Self {
                Self {
                    id: val.id.to_string(),
                    value_type: AssetValueTypeDTO(val.value_type),
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

    mod peers {
        use super::*;
        use iroha_data_model::prelude::{FindAllPeers, Peer, PeerId};

        #[derive(Serialize)]
        pub struct PeerDTO {
            #[serde(flatten)]
            peer_id: PeerId,
        }

        impl From<Peer> for PeerDTO {
            fn from(val: Peer) -> Self {
                Self { peer_id: val.id }
            }
        }

        #[get("")]
        async fn index(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
            let items = data.with_client(|client| client.request(FindAllPeers::new()))??;
            let items: Vec<PeerDTO> = items.into_iter().map(|x| x.into()).collect();
            Ok(web::Json(items))
        }

        pub fn service() -> Scope {
            web::scope("/peers").service(index)
        }
    }

    mod roles {
        use super::*;
        use iroha_data_model::prelude::{FindAllRoles, Role};

        #[derive(Serialize)]
        pub struct RoleDTO {
            #[serde(flatten)]
            inner: Role,
        }

        impl From<Role> for RoleDTO {
            fn from(val: Role) -> Self {
                Self { inner: val }
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

    pub fn server(state: AppState, port: u16) -> color_eyre::Result<actix_server::Server> {
        let state = web::Data::new(state);

        let server = HttpServer::new(move || {
            App::new()
                .app_data(state.clone())
                .wrap(super::logger::TracingLogger::default())
                .service(status::service())
                .service(accounts::service())
                .service(domains::service())
                .service(assets::service())
                .service(peers::service())
                .service(roles::service())
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

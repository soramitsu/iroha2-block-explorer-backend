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
        error::ResponseError, get, http, web, App, HttpResponse, HttpServer, Responder,
    };
    use color_eyre::eyre::{eyre, WrapErr};
    use derive_more::Display;
    use iroha_client::client::Client as IrohaClient;
    use std::sync::Mutex;

    // type WebResult = Result<impl Responder, Into<std::error::Error>>;

    pub struct AppState {
        iroha_client: Mutex<IrohaClient>,
    }

    #[derive(Display, Debug)]
    enum WebError {
        Internal(color_eyre::Report),
    }

    impl ResponseError for WebError {
        fn error_response(&self) -> HttpResponse {
            HttpResponse::build(self.status_code())
                .insert_header(http::header::ContentType::html())
                .body(self.to_string())
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

    #[get("/status")]
    async fn get_status(data: web::Data<AppState>) -> Result<impl Responder, WebError> {
        let status = {
            let lock = data
                .iroha_client
                .lock()
                .map_err(|_| eyre!("mutex is poisoned"))?;
            let status = lock.get_status().wrap_err("failed to get status")?;
            status
        };

        Ok(HttpResponse::Ok().body(format!("{:?}", status)))
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
                .service(get_status)
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

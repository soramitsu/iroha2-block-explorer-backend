mod logger {
    use tracing::{subscriber::set_global_default, Subscriber};
    pub use tracing_actix_web::TracingLogger;
    use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
    use tracing_log::LogTracer;
    use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

    pub use tracing::{error, info};

    /// Compose multiple layers into a `tracing`'s subscriber.
    fn get_subscriber(name: String, env_filter: String) -> impl Subscriber + Send + Sync {
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
        let bunyan_formatter = BunyanFormattingLayer::new(name, std::io::stdout);
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

/// App CLI arguments specific logic
mod args {
    use clap::Parser;
    use color_eyre::{eyre::Context as _, Help as _, Result};
    use iroha_client::{client::Client as IrohaClient, Configuration as IrohaClientConfiguration};

    #[derive(Debug, Parser)]
    #[clap(about = "Iroha 2 Explorer Backend", version, long_about = None)]
    pub struct Args {
        #[clap(short, long, default_value = "4000", env)]
        pub port: u16,

        #[clap(
            short = 'c',
            long,
            default_value = "client_config.json",
            help = "`iroha_client` JSON configuration path"
        )]
        pub client_config: String,

        #[cfg(feature = "dev_actor")]
        #[clap(long, help = "Run actor that fills Iroha with fake data")]
        pub dev_actor: bool,
    }

    impl Args {
        pub fn parse() -> Self {
            Parser::parse()
        }
    }

    #[derive(Debug)]
    pub struct ArgsClientConfig(pub IrohaClientConfiguration);

    impl ArgsClientConfig {
        pub fn load(args: &Args) -> Result<Self> {
            use std::fs::File;

            let file = &args.client_config;

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

    impl TryFrom<ArgsClientConfig> for IrohaClient {
        type Error = color_eyre::Report;

        fn try_from(ArgsClientConfig(cfg): ArgsClientConfig) -> color_eyre::Result<Self> {
            Ok(Self::new(&cfg))
        }
    }
}

/// Web-specific logic - server initialization, endpoints, DTOs etc
mod web;

#[cfg(feature = "dev_actor")]
mod dev_actor;

use std::sync::{Arc, Mutex};

use color_eyre::{eyre::WrapErr, Result};
use iroha_client::client::Client as IrohaClient;

#[actix_web::main]
async fn main() -> Result<()> {
    let args = args::Args::parse();
    let client_config = args::ArgsClientConfig::load(&args)?;

    let client: IrohaClient = client_config
        .try_into()
        .wrap_err("Failed to construct Iroha Client")?;
    let client = Arc::new(Mutex::new(client));

    logger::setup();
    logger::info!("Server is going to listen on {}", args.port);

    #[cfg(feature = "dev_actor")]
    let _dev_actor = if args.dev_actor {
        Some(dev_actor::DevActor::start(client.clone()))
    } else {
        None
    };

    web::server(web::AppState::new(client.clone()), args.port)?
        .await
        .wrap_err("Server run failed")
}

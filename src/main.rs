mod logger {
    use tracing::{subscriber::set_global_default, Subscriber};
    pub use tracing_actix_web::TracingLogger;
    use tracing_log::LogTracer;
    use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

    pub use tracing::{debug, error, info, warn};

    /// Compose multiple layers into a `tracing`'s subscriber.
    fn get_subscriber(default_env_filter: String) -> impl Subscriber + Send + Sync {
        let env_filter_layer = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(default_env_filter));

        let fmt_layer = tracing_subscriber::fmt::layer().compact();

        Registry::default().with(env_filter_layer).with(fmt_layer)
    }

    pub fn setup() {
        LogTracer::init().expect("Failed to set logger");

        let subscriber = get_subscriber("info".into());
        set_global_default(subscriber).expect("Failed to set subscriber");
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

        /// `iroha_client` JSON configuration path
        #[clap(short = 'c', long, default_value = "client_config.json")]
        pub client_config: String,

        /// Run actor that fills Iroha with fake data
        #[cfg(feature = "dev_actor")]
        #[clap(long)]
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
            Self::new(&cfg)
        }
    }
}

/// Web-specific logic - server initialization, endpoints, DTOs etc
mod web;

/// Actix implementation around Iroha Client
mod iroha_client_wrap;

#[cfg(feature = "dev_actor")]
mod dev_actor;

use std::sync::Arc;

use color_eyre::{eyre::WrapErr, Result};
use iroha_client::client::Client as IrohaClient;

#[actix_web::main]
async fn main() -> Result<()> {
    let args = args::Args::parse();
    let client_config = args::ArgsClientConfig::load(&args)?;
    let account_id = client_config.0.account_id.clone();

    let client: IrohaClient = client_config
        .try_into()
        .wrap_err("Failed to construct Iroha Client")?;
    let client = Arc::new(client);

    #[cfg(feature = "dev_actor")]
    let _dev_actor = if args.dev_actor {
        Some(dev_actor::DevActor::start(client.clone(), account_id))
    } else {
        None
    };

    logger::setup();
    logger::info!("Server is going to listen on {}", args.port);

    web::server(web::ServerInitData::new(client.clone()), args.port)?
        .await
        .wrap_err("Server run failed")
}

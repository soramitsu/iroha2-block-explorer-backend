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

/// Module contains app CLI arguments specific logic
mod args {
    use color_eyre::{eyre::Context as _, Help as _, Result};
    use iroha_client::Configuration as IrohaClientConfiguration;
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
}

/// Module contains web-specific logic - server initialization, endpoints, DTOs etc
mod web;

use color_eyre::{eyre::WrapErr, Result};

#[actix_web::main]
async fn main() -> Result<()> {
    let args: args::Args = structopt::StructOpt::from_args();
    let client_config = args::ArgsClientConfig::load(&args)?;

    logger::setup();
    logger::info!("Server is going to listen on {}", args.port);

    web::server(web::AppState::new(&client_config.0), args.port)?
        .await
        .wrap_err("Server run failed")
}

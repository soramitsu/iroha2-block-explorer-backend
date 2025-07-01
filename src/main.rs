// FIXME: there are many truncations which we don't care about _for now_
#![allow(clippy::cast_possible_truncation)]

mod endpoint;

use axum::routing::get;
use axum::{
    extract::{MatchedPath, Request},
    Router,
};
use clap::Parser;
use eyre::{eyre, Context};
use iroha::crypto::{KeyPair, PrivateKey};
use iroha::data_model::account::AccountId;
use iroha_explorer_core::state::State;
use iroha_explorer_iroha_client::Client;
use iroha_explorer_schema::ToriiUrl;
use iroha_explorer_telemetry::{Telemetry, TelemetryConfig};
use iroha_futures::supervisor::{Child, OnShutdown, ShutdownSignal, Supervisor};
use std::collections::BTreeSet;
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use tokio::task::JoinSet;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

const COMPATIBLE_IROHA_VERSION: &str = "v2.0.0-rc.2";
const VERSION: &str = const_str::format!(
    "version={} git_commit_sha={} iroha_compat={}",
    env!("CARGO_PKG_VERSION"),
    env!("VERGEN_GIT_SHA"),
    COMPATIBLE_IROHA_VERSION,
);

#[derive(Debug, Parser)]
#[clap(about = "Iroha 2 Explorer Backend", version = VERSION, long_about = None)]
pub struct Args {
    /// Path to store blocks in
    #[clap(long, short, env = "IROHA_EXPLORER_STORE_DIR")]
    pub store_dir: PathBuf,

    #[command(subcommand)]
    pub command: Subcommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Run the server
    Serve(ServeArgs),
    // #[cfg(debug_assertions)]
    // /// DEBUG-ONLY: run server with test data
    // ServeTest(ServeBaseArgs),
}

#[derive(Parser, Debug)]
pub struct IrohaArgs {
    /// Account ID in a form of `signatory@domain` on which behalf to perform Iroha Queries
    #[clap(long, env = "IROHA_EXPLORER_ACCOUNT")]
    pub account: AccountId,
    /// Multihash of the account's private key
    #[clap(long, env = "IROHA_EXPLORER_ACCOUNT_PRIVATE_KEY")]
    pub account_private_key: PrivateKey,
    /// Iroha Torii URL(s), comma-separated list
    ///
    /// At least one is required.
    #[clap(long, env = "IROHA_EXPLORER_TORII_URLS")]
    pub torii_urls: ArgToriiUrls,
}

impl IrohaArgs {
    fn client(&self) -> Client {
        Client::new(
            self.account.clone(),
            KeyPair::from(self.account_private_key.clone()),
            self.torii_urls.some(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct ArgToriiUrls(BTreeSet<ToriiUrl>);

impl FromStr for ArgToriiUrls {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let urls = s
            .split(',')
            .map(FromStr::from_str)
            .collect::<Result<BTreeSet<ToriiUrl>, _>>()
            .wrap_err("Cannot parse URL(s)")?;
        if urls.is_empty() {
            Err(eyre!("There should be at least one URL"))
        } else {
            Ok(Self(urls))
        }
    }
}

impl ArgToriiUrls {
    fn some(&self) -> ToriiUrl {
        self.0
            .first()
            .cloned()
            .expect("there should be at least one")
    }

    fn all(&self) -> &BTreeSet<ToriiUrl> {
        &self.0
    }
}

#[derive(Parser, Debug)]
pub struct ServeBaseArgs {
    /// Port to run the server on
    #[clap(short, long, default_value = "4000", env = "IROHA_EXPLORER_PORT")]
    port: u16,
    /// IP to run the server on
    #[clap(long, default_value = "127.0.0.1", env = "IROHA_EXPLORER_IP")]
    ip: IpAddr,
}

#[derive(Parser, Debug)]
pub struct ServeArgs {
    #[command(flatten)]
    base: ServeBaseArgs,
    #[command(flatten)]
    iroha: IrohaArgs,
}

#[derive(OpenApi)]
#[openapi(
    nest((path = "/api/v1", api = endpoint::Api)),
    paths(health_check)
)]
struct Api;

// TODO: enable iroha core?
#[cfg(debug_assertions)]
const DEFAULT_LOG: &str = "iroha_explorer=trace,tower_http=debug,sqlx=debug";
#[cfg(not(debug_assertions))]
const DEFAULT_LOG: &str = "info";

#[tokio::main]
async fn main() {
    let args = Args::parse();

    #[cfg(debug_assertions)]
    let fmt_layer = tracing_subscriber::fmt::layer().pretty();
    #[cfg(not(debug_assertions))]
    let fmt_layer = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| DEFAULT_LOG.into()),
        )
        .with(fmt_layer)
        .init();

    match args.command {
        Subcommand::Serve(serve_args) => serve(serve_args, args.store_dir).await,
        // #[cfg(debug_assertions)]
        // Subcommand::ServeTest(args) => serve_test(args).await,
    }
}

async fn serve(args: ServeArgs, store_dir: PathBuf) {
    let client = args.iroha.client();

    let mut sup = Supervisor::new();

    let telemetry = {
        let (tel, fut) = Telemetry::new(TelemetryConfig {
            urls: args.iroha.torii_urls.all().clone(),
        });
        sup.monitor(Child::new(tokio::spawn(fut), OnShutdown::Abort));
        tel
    };

    let state = {
        let (state, fut) = iroha_explorer_core::start(
            store_dir,
            telemetry.clone(),
            client.clone(),
            sup.shutdown_signal(),
        );
        sup.monitor(Child::new(
            tokio::spawn(async move {
                if let Err(error) = fut.await {
                    tracing::error!(%error, "Core supervisor exited with error");
                    // FIXME: communicate error to top-level properly
                    panic!("not okay")
                }
            }),
            OnShutdown::Wait(Duration::from_secs(10)),
        ));
        state
    };

    sup.monitor(Child::new(
        tokio::spawn({
            let state = state.clone();
            let tel = telemetry.clone();
            let signal = sup.shutdown_signal();
            async move {
                do_serve(state, tel, signal, args.base).await;
            }
        }),
        OnShutdown::Wait(Duration::from_secs(5)),
    ));

    sup.setup_shutdown_on_os_signals().unwrap();
    sup.start().await.unwrap()
}

async fn do_serve(
    state: State,
    telemetry: Telemetry,
    shutdown_signal: ShutdownSignal,
    args: ServeBaseArgs,
) {
    // TODO: handle endpoint panics
    let app = Router::new()
        .merge(Scalar::with_url("/api/docs", Api::openapi()))
        .route("/api/health", get(health_check))
        .nest("/api/v1", endpoint::router(state, telemetry))
        .layer(
            TraceLayer::new_for_http()
                // Create our own span for the request and include the matched path. The matched
                // path is useful for figuring out which handler the request was routed to.
                .make_span_with(|req: &Request| {
                    let method = req.method();
                    let uri = req.uri();

                    // axum automatically adds this extension.
                    let matched_path = req
                        .extensions()
                        .get::<MatchedPath>()
                        .map(axum::extract::MatchedPath::as_str);

                    tracing::debug_span!("request", %method, %uri, matched_path)
                })
                // we do logging in `AppError` internally
                .on_failure(()),
        );

    let listener = tokio::net::TcpListener::bind((args.ip, args.port))
        .await
        .unwrap();
    tracing::info!("listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .with_graceful_shutdown(async move { shutdown_signal.receive().await })
        .await
        .unwrap()
}

#[cfg(test)]
async fn serve_test(args: ServeBaseArgs) {
    todo!()
}

/// Health check
#[utoipa::path(get, path = "/api/health", tag = "Misc", responses(
    (status = 200, description = "Explorer is up and running", content_type = "text/plain", example = json!("healthy"))
))]
async fn health_check() -> &'static str {
    "healthy"
}

#[cfg(test)]
fn init_test_logger() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| DEFAULT_LOG.into()),
        )
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;
    use std::net::SocketAddr;
    use std::time::Duration;
    use tokio::spawn;
    use tokio::time::timeout;

    #[test]
    fn cli() {
        use clap::CommandFactory;
        Args::command().debug_assert();
    }

    #[test]
    fn parse_torii_urls() {
        let value: ArgToriiUrls = "http://iroha.tech/1,http://iroha.tech/2".parse().unwrap();
        assert_eq!(value.some(), "http://iroha.tech/1".parse().unwrap());
        assert_eq!(
            value.all().iter().collect::<Vec<_>>(),
            vec![
                &"http://iroha.tech/1".parse::<ToriiUrl>().unwrap(),
                &"http://iroha.tech/2".parse().unwrap(),
            ]
        );

        let _ = ""
            .parse::<ArgToriiUrls>()
            .expect_err("should fail with nothing");
    }

    #[tokio::test]
    async fn serve_test_ok() -> eyre::Result<()> {
        // Uncomment for troubleshooting
        // init_test_logger();

        spawn(serve_test(ServeBaseArgs {
            ip: "127.0.0.1".parse()?,
            port: 9928,
        }));
        let path = |fragment: &str| format!("http://127.0.0.1:9928{fragment}");
        timeout(
            Duration::from_secs(1),
            wait_addr_bind("127.0.0.1:9928".parse()?),
        )
        .await?;

        let client = reqwest::Client::builder().build()?;

        let health = client.get(path("/api/health")).send().await?.text().await?;
        assert_eq!(health, "healthy");

        ensure_status(&client, path("/api/docs"), StatusCode::OK).await;
        ensure_status(&client, path("/api/v1/blocks"), StatusCode::OK).await;
        ensure_status(&client, path("/api/v1/blocks/1"), StatusCode::OK).await;
        ensure_status(&client, path("/api/v1/transactions"), StatusCode::OK).await;
        ensure_status(
            &client,
            path("/api/v1/transactions/bad_hash"),
            StatusCode::BAD_REQUEST,
        )
        .await;
        ensure_status(&client, path("/api/v1/instructions"), StatusCode::OK).await;
        ensure_status(&client, path("/api/v1/domains"), StatusCode::OK).await;
        ensure_status(&client, path("/api/v1/domains/genesis"), StatusCode::OK).await;
        ensure_status(&client, path("/api/v1/accounts"), StatusCode::OK).await;
        ensure_status(&client, path("/api/v1/accounts/ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland"), StatusCode::OK).await;
        ensure_status(&client, path("/api/v1/assets"), StatusCode::OK).await;
        ensure_status(&client, path("/api/v1/assets/rose%23%23ed0120CE7FA46C9DCE7EA4B125E2E36BDB63EA33073E7590AC92816AE1E861B7048B03@wonderland"), StatusCode::OK).await;
        ensure_status(&client, path("/api/v1/nfts"), StatusCode::OK).await;
        ensure_status(
            &client,
            path("/api/v1/nfts/snowflake$wonderland"),
            StatusCode::OK,
        )
        .await;
        ensure_status(&client, path("/api/v1/assets-definitions"), StatusCode::OK).await;
        ensure_status(
            &client,
            path("/api/v1/assets-definitions/cabbage%23garden_of_live_flowers"),
            StatusCode::OK,
        )
        .await;
        ensure_status(
            &client,
            path("/api/v1/telemetry/network"),
            StatusCode::SERVICE_UNAVAILABLE,
        )
        .await;
        ensure_status(&client, path("/api/v1/telemetry/peers"), StatusCode::OK).await;
        ensure_status(
            &client,
            path("/api/v1/telemetry/peers-info"),
            StatusCode::OK,
        )
        .await;
        ensure_status(&client, path("/api/v1/telemetry/live"), StatusCode::OK).await;

        Ok(())
    }

    async fn wait_addr_bind(addr: SocketAddr) {
        while let Err(_) = tokio::net::TcpStream::connect(addr).await {
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
    }

    async fn ensure_status(
        client: &reqwest::Client,
        url: impl reqwest::IntoUrl,
        status: StatusCode,
    ) {
        let url = url.into_url().unwrap();
        let resp = client.get(url.clone()).send().await.unwrap();
        assert_eq!(resp.status(), status, "unexpected status for GET {url}");
    }
}

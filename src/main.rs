#![allow(clippy::module_name_repetitions)]
// FIXME: there are many truncations which we don't care about _for now_
#![allow(clippy::cast_possible_truncation)]

mod database_update;
mod endpoint;
mod iroha_client_wrap;
mod repo;
mod schema;
mod util;

use crate::endpoint::StatusProvider;
use crate::iroha_client_wrap::ClientWrap;
use crate::repo::{scan_iroha, Repo};
use axum::routing::get;
use axum::{
    extract::{MatchedPath, Request},
    Router,
};
use clap::Parser;
use database_update::DatabaseUpdateLoop;
use iroha::crypto::{KeyPair, PrivateKey};
use iroha::data_model::account::AccountId;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{ConnectOptions, Connection};
use std::net::IpAddr;
use std::path::PathBuf;
use tokio::task::JoinSet;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
use url::Url;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

#[derive(Debug, Parser)]
#[clap(about = "Iroha 2 Explorer Backend", version, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Subcommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Run the server
    Serve(ServeArgs),
    #[cfg(debug_assertions)]
    /// DEBUG-ONLY: run server with test data
    ServeTest(ServeBaseArgs),
    /// Scan Iroha into an `SQLite` database and save it to file
    Scan(ScanArgs),
}

#[derive(Parser, Debug)]
pub struct IrohaCredentialsArgs {
    /// Account ID in a form of `signatory@domain` on which behalf to perform Iroha Queries
    #[clap(long, env = "IROHA_EXPLORER_ACCOUNT")]
    pub account: AccountId,
    /// Multihash of the account's private key
    #[clap(long, env = "IROHA_EXPLORER_ACCOUNT_PRIVATE_KEY")]
    pub account_private_key: PrivateKey,
    /// Iroha Torii URL
    #[clap(long, env = "IROHA_EXPLORER_TORII_URL")]
    pub torii_url: Url,
}

impl IrohaCredentialsArgs {
    fn client(self) -> ClientWrap {
        ClientWrap::new(
            self.account,
            KeyPair::from(self.account_private_key),
            self.torii_url,
        )
    }
}

#[derive(Parser, Debug)]
pub struct ScanArgs {
    #[command(flatten)]
    creds: IrohaCredentialsArgs,
    /// Path to `SQLite` database to scan Iroha to
    out_file: PathBuf,
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
    creds: IrohaCredentialsArgs,
}

#[derive(OpenApi)]
#[openapi(
    nest((path = "/api/v1", api = endpoint::Api)),
    paths(health_check)
)]
struct Api;

#[cfg(debug_assertions)]
const DEFAULT_LOG: &str = "iroha_explorer=debug,tower_http=debug,sqlx=debug";
#[cfg(not(debug_assertions))]
const DEFAULT_LOG: &str = "info";

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| DEFAULT_LOG.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // let key_pair = KeyPair::from(args.account_private_key);
    // let iroha_client = ClientWrap::new(args.account, key_pair, args.torii_url);

    match args.command {
        Subcommand::Serve(args) => serve(args).await,
        #[cfg(debug_assertions)]
        Subcommand::ServeTest(args) => serve_test(args).await,
        Subcommand::Scan(args) => scan(args).await.unwrap(),
    }
}

async fn serve(args: ServeArgs) {
    let repo = Repo::new(None);
    let client = args.creds.client();

    let mut set = JoinSet::<()>::new();
    set.spawn({
        let repo = repo.clone();
        let client = client.clone();
        async move {
            do_serve(repo, StatusProvider::Iroha(client), args.base).await;
        }
    });
    set.spawn(async move {
        DatabaseUpdateLoop::new(repo, client).run().await;
    });
    set.join_all().await;
}

#[cfg(debug_assertions)]
async fn serve_test(args: ServeBaseArgs) {
    let repo = Repo::new(None);
    fill_repo_with_test_data(&repo).await.unwrap();
    tracing::info!("test data is ready");

    let mut set = JoinSet::<()>::new();
    set.spawn(async move {
        do_serve(repo.clone(), StatusProvider::None, args).await;
    });
    set.join_all().await;
}

async fn do_serve(repo: Repo, status_provider: StatusProvider, args: ServeBaseArgs) {
    // TODO: handle endpoint panics
    let app = Router::new()
        .merge(Scalar::with_url("/api/docs", Api::openapi()))
        .route("/api/health", get(health_check))
        .nest("/api/v1", endpoint::router(repo, status_provider))
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
    tracing::debug!("listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap()
}

/// Health check
#[utoipa::path(get, path = "/api/health")]
async fn health_check() -> &'static str {
    "healthy"
}

#[cfg(debug_assertions)]
async fn fill_repo_with_test_data(repo: &Repo) -> eyre::Result<()> {
    let mut conn = SqliteConnectOptions::new()
        .in_memory(true)
        .connect()
        .await?;
    sqlx::query(include_str!("./repo/test_dump.sql"))
        .execute(&mut conn)
        .await?;
    repo.swap(conn).await;
    Ok(())
}

async fn scan(args: ScanArgs) -> eyre::Result<()> {
    let mut conn = SqliteConnectOptions::new()
        .filename(&args.out_file)
        .create_if_missing(true)
        .connect()
        .await?;
    scan_iroha(&args.creds.client(), &mut conn).await?;
    conn.close().await?;
    tracing::info!(?args.out_file, "written");
    Ok(())
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

    #[tokio::test]
    async fn serve_test_ok() -> eyre::Result<()> {
        // Uncomment for troubleshooting
        // tracing_subscriber::registry()
        //     .with(
        //         tracing_subscriber::EnvFilter::try_from_default_env()
        //             .unwrap_or_else(|_| DEFAULT_LOG.into()),
        //     )
        //     .with(tracing_subscriber::fmt::layer())
        //     .init();

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
        ensure_status(&client, path("/api/v1/status"), StatusCode::NOT_IMPLEMENTED).await;

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

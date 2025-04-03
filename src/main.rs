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
    #[clap(long, env)]
    pub account: AccountId,
    /// Multihash of the account's private key
    #[clap(long, env)]
    pub account_private_key: PrivateKey,
    /// Iroha Torii URL
    #[clap(long, env)]
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
    #[clap(short, long, default_value = "4000", env)]
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

// TODO: utoipa v5-alpha supports nested OpenApi impls (we use v4 now). Use it for `endpoint` module.
#[derive(OpenApi)]
#[openapi(
    paths(
        endpoint::accounts_index,
        endpoint::accounts_show,
        endpoint::assets_index,
        endpoint::assets_show,
        endpoint::nfts_index,
        endpoint::nfts_show,
        endpoint::assets_definitions_index,
        endpoint::assets_definitions_show,
        endpoint::domains_index,
        endpoint::domains_show,
        endpoint::blocks_index,
        endpoint::blocks_show,
        endpoint::transactions_index,
        endpoint::transactions_show,
        endpoint::instructions_index,
        endpoint::status_show
    ),
    components(schemas(
        schema::Domain,
        schema::DomainId,
        schema::Asset,
        schema::AssetId,
        schema::AssetDefinition,
        schema::AssetDefinitionId,
        schema::NftId,
        schema::Nft,
        schema::Mintable,
        schema::Account,
        schema::AccountId,
        schema::IpfsPath,
        schema::Metadata,
        schema::Pagination,
        schema::DomainsPage,
        schema::Block,
        schema::Executable,
        schema::Instruction,
        schema::TransactionStatus,
        schema::TransactionBase,
        schema::TransactionDetailed,
        schema::TransactionRejectionReason,
        schema::Status,
        schema::Instruction,
        schema::InstructionKind,
        schema::TimeStamp,
        schema::BigInt,
        schema::Decimal,
        schema::Hash,
        schema::Signature,
        schema::Duration,
    ))
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(
            // TODO: configure filter via env + use different defaults for debug and release
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "iroha_explorer=debug,tower_http=debug,sqlx=debug".into()),
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
        .merge(Scalar::with_url("/api/docs", ApiDoc::openapi()))
        .route("/api/health", get(|| async { "healthy" }))
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
    tracing::debug!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap()
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

    #[test]
    fn cli() {
        use clap::CommandFactory;
        Args::command().debug_assert();
    }
}

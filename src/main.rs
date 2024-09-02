#![allow(clippy::module_name_repetitions)]

mod endpoint;
mod iroha;
mod repo;
mod schema;
mod util;

use crate::iroha::Client;

use crate::repo::Repo;
use axum::{
    extract::{MatchedPath, Request},
    Router,
};
use clap::Parser;
use iroha_crypto::{KeyPair, PrivateKey};
use iroha_data_model::account::AccountId;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::ConnectOptions;
use tower_http::trace::TraceLayer;
use tracing::log::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
use url::Url;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

#[derive(Debug, Parser)]
#[clap(about = "Iroha 2 Explorer Backend", version, long_about = None)]
pub struct Args {
    /// Port to run the server on
    #[clap(short, long, default_value = "4000", env)]
    pub port: u16,

    /// Account ID in a form of `signatory@domain` on which behalf to perform Iroha Queries
    #[clap(long, env)]
    pub account: AccountId,

    /// Multihash of the account's private key
    #[clap(long, env)]
    pub account_private_key: PrivateKey,

    /// Iroha Torii URL
    #[clap(long, env)]
    pub torii_url: Url,

    /// Path to SQLite database
    #[clap(long, env)]
    pub database: String,
}

// TODO: utoipa v5-alpha supports nested OpenApi impls (we use v4 now). Use it for `endpoint` module.
#[derive(OpenApi)]
#[openapi(
    paths(
        endpoint::accounts_index,
        endpoint::accounts_show,
        endpoint::assets_index,
        endpoint::assets_show,
        endpoint::assets_definitions_index,
        endpoint::assets_definitions_show,
        endpoint::domains_index,
        endpoint::domains_show,
        endpoint::blocks_index,
        endpoint::blocks_show,
        endpoint::transactions_index,
        endpoint::transactions_show,
        endpoint::instructions_index
    ),
    components(schemas(
        schema::Domain,
        schema::DomainId,
        schema::Asset,
        schema::AssetId,
        schema::AssetDefinition,
        schema::AssetDefinitionId,
        schema::AssetType,
        schema::AssetValue,
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
        schema::TransactionBase,
        schema::TransactionInList,
        schema::TransactionDetailed,
        schema::TransactionRejectionReason,
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
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "iroha_explorer=debug,tower_http=debug,sqlx=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let key_pair = KeyPair::from(args.account_private_key);
    let iroha = Client::new(args.account, key_pair, args.torii_url);

    let opts = SqliteConnectOptions::new()
        .filename(args.database)
        .log_statements(LevelFilter::Debug);
    let repo = Repo::new(opts).await.unwrap();

    // TODO: handle endpoint panics
    let app = Router::new()
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .nest("/api/v1", endpoint::router(iroha, repo))
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
                        .map(|matched_path| matched_path.as_str());

                    tracing::debug_span!("request", %method, %uri, matched_path)
                })
                // we do logging in `AppError` internally
                .on_failure(()),
        );

    let listener = tokio::net::TcpListener::bind(("localhost", args.port))
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

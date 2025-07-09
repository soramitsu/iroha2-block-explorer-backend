use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{sse, IntoResponse},
    routing::get,
    Json, Router,
};
use futures_util::Stream;
use futures_util::StreamExt;
use serde::Deserialize;
use utoipa::{IntoParams, OpenApi};

use iroha_explorer_core::{query, state};
use iroha_explorer_schema::{self as schema, Page, PaginationQueryParams};
use iroha_explorer_telemetry::Telemetry;

#[derive(Clone)]
pub struct AppState {
    telemetry: Telemetry,
    state: state::State,
}

impl AppState {
    // TODO: remove result?
    async fn query(&self) -> Result<query::QueryExecutor, AppError> {
        let guard = self.state.acquire_guard().await;
        Ok(query::QueryExecutor::new(guard))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("query-related error: {0}")]
    Query(#[from] query::Error),
    #[error("Network status is not yet available")]
    NetworkStatusNotAvailable,
    #[error("{0}")]
    Other(#[from] eyre::Report),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::Query(err @ query::Error::NotFound { .. }) => {
                (StatusCode::NOT_FOUND, format!("{err}")).into_response()
            }
            AppError::Query(err @ query::Error::BadReversePagination(_)) => {
                (StatusCode::BAD_REQUEST, format!("{err}")).into_response()
            }
            AppError::Query(err @ query::Error::BadParams { .. }) => {
                (StatusCode::BAD_REQUEST, format!("{err}")).into_response()
            }
            AppError::NetworkStatusNotAvailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Network status is not yet available. Please try again later.",
            )
                .into_response(),
            AppError::Other(report) => {
                tracing::error!(%report, "other error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong").into_response()
            }
        }
    }
}

#[derive(IntoParams, Deserialize)]
struct DomainsIndexFilter {
    owned_by: Option<schema::AccountId>,
}

/// List domains
#[utoipa::path(
    get,
    path = "/domains",
    tags = ["Blockchain entities"],
    responses(
        (status = 200, description = "OK", body = Page<schema::Domain>)
    ),
    params(schema::PaginationQueryParams, DomainsIndexFilter)
)]
async fn domains_index(
    State(state): State<AppState>,
    Query(pagination): Query<schema::PaginationQueryParams>,
    Query(filter): Query<DomainsIndexFilter>,
) -> Result<Json<schema::Page<schema::Domain>>, AppError> {
    let domains = state
        .query()
        .await?
        .domains_index(filter.owned_by.as_ref(), &pagination);

    Ok(Json(domains))
}

/// Find a domain
#[utoipa::path(get, path = "/domains/{id}",
    tags = ["Blockchain entities"],
    responses(
    (status = 200, description = "Domain Found", body = schema::Domain),
    (status = 404, description = "Domain Not Found")
), params(("id" = schema::DomainId, description = "Domain ID", example = "genesis")))]
async fn domains_show(
    State(state): State<AppState>,
    Path(id): Path<schema::DomainId>,
) -> Result<Json<schema::Domain>, AppError> {
    let domain = state.query().await?.domains_show(&id)?;
    Ok(Json(schema::Domain::from(domain)))
}

/// List blocks
// TODO: describe page number
#[utoipa::path(
    get,
    path = "/blocks",
    tags = ["Blockchain entities"],
    responses(
        (status = 200, description = "OK", body = [schema::Block]),
    ),
    params(schema::PaginationQueryParams)
)]
async fn blocks_index(
    State(state): State<AppState>,
    Query(pagination): Query<schema::PaginationQueryParams>,
) -> Result<Json<Page<schema::Block>>, AppError> {
    let blocks = state.query().await?.blocks_index(&pagination)?;
    Ok(Json(blocks))
}

/// Find a block by its hash/height
#[utoipa::path(
    get,
    path = "/blocks/{height_or_hash}",
    tags = ["Blockchain entities"],
    params(
        ("height_or_hash", description = "Height or hash of the block", example = "12")
    ),
    responses(
        (status = 200, description = "OK", body = schema::Block),
        (status = 404, description = "Block Not Found")
    )
)]
async fn blocks_show(
    State(state): State<AppState>,
    Path(height_or_hash): Path<schema::BlockHeightOrHash>,
) -> Result<Json<schema::Block>, AppError> {
    let block = state.query().await?.blocks_show(&height_or_hash)?;
    Ok(Json(block))
}

/// List transactions
#[utoipa::path(
    get,
    path = "/transactions",
    tags = ["Blockchain entities"],
    params(schema::PaginationQueryParams, schema::TransactionsIndexFilter),
    responses(
        (status = 200, description = "OK", body = Page<schema::TransactionBase>)
    )
)]
async fn transactions_index(
    State(state): State<AppState>,
    Query(pagination): Query<schema::PaginationQueryParams>,
    Query(filter): Query<schema::TransactionsIndexFilter>,
) -> Result<Json<Page<schema::TransactionBase>>, AppError> {
    let page = state
        .query()
        .await?
        .transactions_index(&filter, &pagination)?;
    Ok(Json(page))
}

/// Find a transaction by its hash
#[utoipa::path(get, path = "/transactions/{hash}",
    tags = ["Blockchain entities"],
    params(
    ("hash" = schema::Hash, description = "Hash of the transaction", example = "9FC55BD948D0CDE0838F6D86FA069A258F033156EE9ACEF5A5018BC9589473F3")
), responses(
    (status = 200, description = "Transaction Found", body = schema::TransactionDetailed),
    (status = 404, description = "Transaction Not Found")
))]
async fn transactions_show(
    State(state): State<AppState>,
    Path(hash): Path<schema::Hash>,
) -> Result<Json<schema::TransactionDetailed>, AppError> {
    let tx = state.query().await?.transactions_show(&hash.0)?;
    Ok(Json(tx.into()))
}

/// List accounts
#[utoipa::path(
    get,
    path = "/accounts",
    tags = ["Blockchain entities"],
    params(schema::PaginationQueryParams, schema::AccountsIndexFilter),
    responses(
        (status = 200, description = "OK", body = [schema::Account])
    )
)]
async fn accounts_index(
    State(state): State<AppState>,
    Query(pagination): Query<schema::PaginationQueryParams>,
    Query(filter): Query<schema::AccountsIndexFilter>,
) -> Result<Json<Page<schema::Account>>, AppError> {
    let page = state.query().await?.accounts_index(&filter, &pagination)?;
    Ok(Json(page))
}

/// Find an account
#[utoipa::path(get, path = "/accounts/{id}",
    tags = ["Blockchain entities"],
    responses(
    (status = 200, description = "Found", body = schema::Account),
    (status = 404, description = "Not Found")
), params(("id" = schema::AccountId, description = "Account ID")))]
async fn accounts_show(
    State(state): State<AppState>,
    Path(id): Path<schema::AccountId>,
) -> Result<Json<schema::Account>, AppError> {
    Ok(Json(state.query().await?.accounts_show(&id)?))
}

/// List asset definitions
#[utoipa::path(
    get,
    path = "/assets-definitions",
    tags = ["Blockchain entities"],
    params(schema::PaginationQueryParams, schema::AssetDefinitionsIndexFilter),
    responses(
        (status = 200, description = "OK", body = [schema::AssetDefinition])
    )
)]
async fn assets_definitions_index(
    State(state): State<AppState>,
    Query(pagination): Query<schema::PaginationQueryParams>,
    Query(filter): Query<schema::AssetDefinitionsIndexFilter>,
) -> Result<Json<Page<schema::AssetDefinition>>, AppError> {
    let page = state.query().await?.asset_defs_index(&filter, &pagination);
    Ok(Json(page))
}

/// Find an asset definition
#[utoipa::path(get, path = "/assets-definitions/{id}",
    tags = ["Blockchain entities"],
    responses(
    (status = 200, description = "Found", body = schema::AssetDefinition),
    (status = 404, description = "Not Found")
), params(("id" = schema::AssetDefinitionId, description = "Asset Definition ID")))]
async fn assets_definitions_show(
    State(state): State<AppState>,
    Path(id): Path<schema::AssetDefinitionId>,
) -> Result<Json<schema::AssetDefinition>, AppError> {
    let item = state.query().await?.asset_defs_show(&id)?;
    Ok(Json(item))
}

/// List assets
#[utoipa::path(
    get,
    path = "/assets",
    tags = ["Blockchain entities"],
    params(schema::PaginationQueryParams, schema::AssetsIndexFilter),
    responses(
        (status = 200, description = "OK", body = [schema::Asset])
    )
)]
async fn assets_index(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQueryParams>,
    Query(filter): Query<schema::AssetsIndexFilter>,
) -> Result<Json<Page<schema::Asset>>, AppError> {
    let page = state.query().await?.assets_index(&filter, &pagination)?;
    Ok(Json(page))
}

/// Find an asset
#[utoipa::path(get, path = "/assets/{id}",
    tags = ["Blockchain entities"],
    responses(
    (status = 200, description = "Found", body = schema::Asset),
    (status = 404, description = "Not Found")
), params(("id" = schema::AssetId, description = "Asset ID")))]
async fn assets_show(
    State(state): State<AppState>,
    Path(id): Path<schema::AssetId>,
) -> Result<Json<schema::Asset>, AppError> {
    Ok(Json(state.query().await?.assets_show(&id)?))
}

/// List NFTs
#[utoipa::path(
    get,
    path = "/nfts",
    tags = ["Blockchain entities"],
    params(schema::PaginationQueryParams, schema::AssetDefinitionsIndexFilter),
    responses(
        (status = 200, description = "OK", body = [schema::Nft])
    )
)]
async fn nfts_index(
    State(state): State<AppState>,
    Query(pagination): Query<schema::PaginationQueryParams>,
    Query(filter): Query<schema::AssetDefinitionsIndexFilter>,
) -> Result<Json<Page<schema::Nft>>, AppError> {
    let page = state.query().await?.nfts_index(&filter, &pagination);
    Ok(Json(page))
}

/// Find an asset definition
#[utoipa::path(get, path = "/nfts/{id}",
    tags = ["Blockchain entities"],
    responses(
    (status = 200, description = "Found", body = schema::Nft),
    (status = 404, description = "Not Found")
), params(("id" = schema::NftId, description = "Asset Definition ID")))]
async fn nfts_show(
    State(state): State<AppState>,
    Path(id): Path<schema::NftId>,
) -> Result<Json<schema::Nft>, AppError> {
    let item = state.query().await?.nfts_show(&id)?;
    Ok(Json(item))
}

/// List instructions
#[utoipa::path(
    get,
    path = "/instructions",
    tags = ["Blockchain entities"],
    params(PaginationQueryParams, schema::InstructionsIndexFilter),
    responses(
        (status = 200, description = "OK", body = [schema::Instruction])
    )
)]
async fn instructions_index(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQueryParams>,
    Query(filter): Query<schema::InstructionsIndexFilter>,
) -> Result<Json<Page<schema::Instruction>>, AppError> {
    let items = state
        .query()
        .await?
        .instructions_index(&filter, &pagination)?;
    Ok(Json(items))
}

/// Get overall network telemetry
#[utoipa::path(
    get,
    path = "/network",
    tags = ["Telemetry"],
    responses(
        (status = 200, description = "OK", body = schema::NetworkStatus),
        (status = 503, description = "Explorer has not yet gathered sufficient data to serve this request")
    )
)]
pub async fn telemetry_network(
    State(state): State<AppState>,
) -> Result<Json<schema::NetworkStatus>, AppError> {
    let data = state
        .telemetry
        .network()
        .await?
        .ok_or(AppError::NetworkStatusNotAvailable)?;
    Ok(Json(data))
}

/// Get telemetry for all connected peers
#[utoipa::path(
    get,
    path = "/peers",
    tags = ["Telemetry"],
    responses(
        (status = 200, description = "OK", body = [schema::PeerStatus])
    )
)]
pub async fn telemetry_peers(
    State(state): State<AppState>,
) -> Result<Json<Vec<schema::PeerStatus>>, AppError> {
    let data = state.telemetry.peers().await?;
    Ok(Json(data))
}

/// Receive live updates of telemetry
#[utoipa::path(
    get,
    path = "/live",
    tags = ["Telemetry"],
    responses(
        (
            status = 200,
            description = "OK, stream is ready",
            content_type = "text/event-stream",
            body = schema::TelemetryStreamMessage
        )
    )
)]
pub async fn telemetry_live(
    State(state): State<AppState>,
) -> Result<sse::Sse<impl Stream<Item = Result<sse::Event, axum::Error>>>, AppError> {
    let stream = state
        .telemetry
        .live()
        .await?
        .map(|data| sse::Event::default().json_data(data));
    Ok(sse::Sse::new(stream).keep_alive(sse::KeepAlive::default()))
}

/// Get static telemetry information about peers
#[utoipa::path(
    get,
    path = "/peers-info",
    tags = ["Telemetry"],
    responses(
        (status = 200, description = "OK", body = [schema::PeerInfo])
    )
)]
pub async fn telemetry_peers_info(
    State(state): State<AppState>,
) -> Result<Json<Vec<schema::PeerInfo>>, AppError> {
    let data = state.telemetry.peers_info().await?;
    Ok(Json(data))
}

pub fn router(state: state::State, telemetry: Telemetry) -> Router {
    Router::new()
        .route("/domains", get(domains_index))
        .route("/domains/{:id}", get(domains_show))
        .route("/accounts", get(accounts_index))
        .route("/accounts/{:id}", get(accounts_show))
        .route("/assets-definitions", get(assets_definitions_index))
        .route("/assets-definitions/{:id}", get(assets_definitions_show))
        .route("/assets", get(assets_index))
        .route("/assets/{:id}", get(assets_show))
        .route("/nfts", get(nfts_index))
        .route("/nfts/{:id}", get(nfts_show))
        .route("/blocks", get(blocks_index))
        .route("/blocks/{:height_or_hash}", get(blocks_show))
        .route("/transactions", get(transactions_index))
        .route("/transactions/{:hash}", get(transactions_show))
        .route("/instructions", get(instructions_index))
        .route("/telemetry/network", get(telemetry_network))
        .route("/telemetry/peers", get(telemetry_peers))
        .route("/telemetry/peers-info", get(telemetry_peers_info))
        .route("/telemetry/live", get(telemetry_live))
        .with_state(AppState { state, telemetry })
}

// TODO: add new paths
#[derive(OpenApi)]
#[openapi(
    paths(
        accounts_index,
        accounts_show,
        assets_index,
        assets_show,
        nfts_index,
        nfts_show,
        assets_definitions_index,
        assets_definitions_show,
        domains_index,
        domains_show,
        blocks_index,
        blocks_show,
        transactions_index,
        transactions_show,
        instructions_index,
    ),
    nest((path = "/telemetry", api = TelemetryApi)),
    tags(
        (name = "Blockchain entities", description = "Routes serving blockchain entities such as blocks, transactions, domains etc"),
        (name = "Telemetry", description = "Routes serving network and peers telemetry data")
    )
)]
pub struct Api;

#[derive(OpenApi)]
#[openapi(paths(
    telemetry_network,
    telemetry_peers,
    telemetry_peers_info,
    telemetry_live
))]
struct TelemetryApi;

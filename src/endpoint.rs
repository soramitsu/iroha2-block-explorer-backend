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

use crate::schema::{Page, PaginationQueryParams, TransactionStatus};
use crate::telemetry::Telemetry;
use crate::{
    repo::{self, Repo},
    schema,
};

#[derive(Clone)]
pub struct AppState {
    telemetry: Telemetry,
    repo: Repo,
}

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("database-related error: {0}")]
    Repo(#[from] repo::Error),
    #[error("Network status is not yet available")]
    NetworkStatusNotAvailable,
    #[error("{0}")]
    Other(#[from] eyre::Report),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::Repo(repo::Error::Pagination(x)) => {
                (StatusCode::BAD_REQUEST, format!("{x}")).into_response()
            }
            AppError::Repo(repo::Error::Sqlx(sqlx::Error::RowNotFound)) => {
                (StatusCode::NOT_FOUND, "Not Found").into_response()
            }
            AppError::Repo(repo::Error::Sqlx(err)) => {
                tracing::error!(%err, "sqlx error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong").into_response()
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
        .repo
        .list_domains(repo::ListDomainParams {
            pagination,
            owned_by: filter.owned_by.map(|x| x.0),
        })
        .await?
        .map(schema::Domain::from);

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
    let domain = state.repo.find_domain(id.0).await?;
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
    let blocks = state
        .repo
        .list_blocks(pagination)
        .await?
        .map(schema::Block::from);
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
    let params = match height_or_hash {
        schema::BlockHeightOrHash::Height(height) => {
            repo::FindBlockParams::Height(height.get() as u32)
        }
        schema::BlockHeightOrHash::Hash(hash) => repo::FindBlockParams::Hash(hash),
    };

    let block = state.repo.find_block(params).await?;
    Ok(Json(block.into()))
}

#[derive(Deserialize, IntoParams)]
struct TransactionsIndexFilter {
    /// Select by authority
    authority: Option<schema::AccountId>,
    /// Select by block
    block: Option<u64>,
    /// Filter by transaction status
    status: Option<schema::TransactionStatus>,
}

/// List transactions
#[utoipa::path(
    get,
    path = "/transactions",
    tags = ["Blockchain entities"],
    params(schema::PaginationQueryParams, TransactionsIndexFilter),
    responses(
        (status = 200, description = "OK", body = Page<schema::TransactionBase>)
    )
)]
async fn transactions_index(
    State(state): State<AppState>,
    Query(pagination): Query<schema::PaginationQueryParams>,
    Query(filter): Query<TransactionsIndexFilter>,
) -> Result<Json<Page<schema::TransactionBase>>, AppError> {
    let page = state
        .repo
        .list_transactions(repo::ListTransactionsParams {
            pagination,
            block: filter.block,
            authority: filter.authority.map(|x| x.0),
            status: filter.status,
        })
        .await?
        .map(schema::TransactionBase::from);
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
    let tx = state.repo.find_transaction_by_hash(hash.0).await?;
    Ok(Json(tx.into()))
}

#[derive(IntoParams, Deserialize)]
struct AccountsIndexFilter {
    /// Select accounts owning specified asset
    with_asset: Option<schema::AssetDefinitionId>,
    /// Select accounts from specified domain
    domain: Option<schema::DomainId>,
}

/// List accounts
#[utoipa::path(
    get,
    path = "/accounts",
    tags = ["Blockchain entities"],
    params(schema::PaginationQueryParams, AccountsIndexFilter),
    responses(
        (status = 200, description = "OK", body = [schema::Account])
    )
)]
async fn accounts_index(
    State(state): State<AppState>,
    Query(pagination): Query<schema::PaginationQueryParams>,
    Query(filter): Query<AccountsIndexFilter>,
) -> Result<Json<Page<schema::Account>>, AppError> {
    let page = state
        .repo
        .list_accounts(repo::ListAccountsParams {
            pagination,
            with_asset: filter.with_asset.map(|x| x.0),
            domain: filter.domain.map(|x| x.0),
        })
        .await?
        .map(schema::Account::from);
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
    Ok(Json(state.repo.find_account(id.0).await?.into()))
}

#[derive(IntoParams, Deserialize)]
struct AssetDefinitionsIndexFilter {
    /// Filter by domain
    domain: Option<schema::DomainId>,
    /// Filter by owner
    owned_by: Option<schema::AccountId>,
}

/// List asset definitions
#[utoipa::path(
    get,
    path = "/assets-definitions",
    tags = ["Blockchain entities"],
    params(schema::PaginationQueryParams, AssetDefinitionsIndexFilter),
    responses(
        (status = 200, description = "OK", body = [schema::AssetDefinition])
    )
)]
async fn assets_definitions_index(
    State(state): State<AppState>,
    Query(pagination): Query<schema::PaginationQueryParams>,
    Query(filter): Query<AssetDefinitionsIndexFilter>,
) -> Result<Json<Page<schema::AssetDefinition>>, AppError> {
    let page = state
        .repo
        .list_assets_definitions(repo::ListAssetDefinitionParams {
            pagination,
            domain: filter.domain.map(|x| x.0),
            owned_by: filter.owned_by.map(|x| x.0),
        })
        .await?
        .map(schema::AssetDefinition::from);
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
    let item = state.repo.find_asset_definition(id.0).await?.into();
    Ok(Json(item))
}

#[derive(Deserialize, IntoParams)]
struct AssetsIndexFilter {
    /// Filter by an owning account
    owned_by: Option<schema::AccountId>,
    /// Filter by asset definition
    definition: Option<schema::AssetDefinitionId>,
}

/// List assets
#[utoipa::path(
    get,
    path = "/assets",
    tags = ["Blockchain entities"],
    params(schema::PaginationQueryParams, AssetsIndexFilter),
    responses(
        (status = 200, description = "OK", body = [schema::Asset])
    )
)]
async fn assets_index(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQueryParams>,
    Query(filter): Query<AssetsIndexFilter>,
) -> Result<Json<Page<schema::Asset>>, AppError> {
    let page = state
        .repo
        .list_assets(repo::ListAssetsParams {
            pagination,
            owned_by: filter.owned_by.map(|x| x.0),
            definition: filter.definition.map(|x| x.0),
        })
        .await?
        .map(schema::Asset::from);
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
    let item = state.repo.find_asset(id.0).await?;
    Ok(Json(item.into()))
}

/// List NFTs
#[utoipa::path(
    get,
    path = "/nfts",
    tags = ["Blockchain entities"],
    params(schema::PaginationQueryParams, AssetDefinitionsIndexFilter),
    responses(
        (status = 200, description = "OK", body = [schema::Nft])
    )
)]
async fn nfts_index(
    State(state): State<AppState>,
    Query(pagination): Query<schema::PaginationQueryParams>,
    Query(filter): Query<AssetDefinitionsIndexFilter>,
) -> Result<Json<Page<schema::Nft>>, AppError> {
    let page = state
        .repo
        .list_nfts(repo::ListNftsParams {
            pagination,
            domain: filter.domain.map(|x| x.0),
            owned_by: filter.owned_by.map(|x| x.0),
        })
        .await?
        .map(schema::Nft::from);
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
    let item = state.repo.find_nft(id.0).await?.into();
    Ok(Json(item))
}

#[derive(Deserialize, IntoParams)]
struct InstructionsIndexFilter {
    transaction_hash: Option<schema::Hash>,
    /// Select by transaction status
    transaction_status: Option<TransactionStatus>,
    /// Select by block
    block: Option<u64>,
    /// Filter by a kind of instruction
    kind: Option<schema::InstructionKind>,
    /// Filter by the creator of the parent transaction
    authority: Option<schema::AccountId>,
}

/// List instructions
#[utoipa::path(
    get,
    path = "/instructions",
    tags = ["Blockchain entities"],
    params(PaginationQueryParams, InstructionsIndexFilter),
    responses(
        (status = 200, description = "OK", body = [schema::Instruction])
    )
)]
async fn instructions_index(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQueryParams>,
    Query(filter): Query<InstructionsIndexFilter>,
) -> Result<Json<Page<schema::Instruction>>, AppError> {
    let items = state
        .repo
        .list_instructions(repo::ListInstructionParams {
            pagination,
            transaction_hash: filter.transaction_hash.map(|x| x.0),
            transaction_status: filter.transaction_status,
            block: filter.block,
            kind: filter.kind,
            authority: filter.authority.map(|x| x.0),
        })
        .await?
        .map(schema::Instruction::from);
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

pub fn router(repo: Repo, telemetry: Telemetry) -> Router {
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
        .with_state(AppState { repo, telemetry })
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

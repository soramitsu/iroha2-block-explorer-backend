use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use iroha_data_model::{query::error::QueryExecutionFail, ValidationFail};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::iroha::{Client, Error as IrohaError};
use crate::schema::Page;
use crate::{
    repo::{self, Repo},
    schema,
};

#[derive(Clone)]
struct AppState {
    iroha: Arc<Client>,
    repo: Repo,
}

#[derive(thiserror::Error, Debug)]
enum AppError {
    #[error("failed to perform Iroha query: {0}")]
    IrohaClientError(#[from] crate::iroha::Error),
    #[error("not found")]
    NotFound,
    // #[error("invalid pagination: {0}")]
    // BadPage(#[from] ReversePaginationError),
    #[error("database-related error: {0}")]
    Repo(#[from] repo::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::IrohaClientError(IrohaError::QueryValidationFail {
                reason: ValidationFail::QueryFailed(QueryExecutionFail::Find(error)),
            }) => (
                StatusCode::NOT_FOUND,
                format!("Iroha couldn't find requested resource: {error}"),
            )
                .into_response(),
            AppError::IrohaClientError(err) => {
                tracing::error!(%err, "iroha client error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong").into_response()
            }
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not found").into_response(),
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
    path = "/api/v1/domains",
    responses(
        (status = 200, description = "OK", body = schema::DomainsPage)
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
#[utoipa::path(get, path = "/api/v1/domains/{id}", responses(
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
    path = "/api/v1/blocks",
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
    path = "/api/v1/blocks/{height_or_hash}",
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
    /// Select by block hash
    block_hash: Option<schema::Hash>,
}

/// List transactions
#[utoipa::path(
    get,
    path = "/api/v1/transactions",
    params(schema::PaginationQueryParams, TransactionsIndexFilter),
    responses(
        (status = 200, description = "OK", body = [schema::Transaction])
    )
)]
async fn transactions_index(
    State(state): State<AppState>,
    Query(pagination): Query<schema::PaginationQueryParams>,
    Query(filter): Query<TransactionsIndexFilter>,
) -> Result<Json<Page<schema::TransactionInList>>, AppError> {
    let page = state
        .repo
        .list_transactions(repo::ListTransactionsParams {
            pagination,
            block_hash: filter.block_hash.map(|x| x.0),
            authority: filter.authority.map(|x| x.0),
        })
        .await?
        .map(schema::TransactionInList::from);
    Ok(Json(page))
}

/// Find a transaction by its hash
#[utoipa::path(get, path = "/api/v1/transactions/{hash}", params(
    ("hash" = schema::Hash, description = "Hash of the transaction", example = "9FC55BD948D0CDE0838F6D86FA069A258F033156EE9ACEF5A5018BC9589473F3")
), responses(
    (status = 200, description = "Transaction Found", body = schema::TransactionWithHash),
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
    path = "/api/v1/accounts",
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
#[utoipa::path(get, path = "/api/v1/accounts/{id}", responses(
    (status = 200, description = "Found", body = schema::Account),
    (status = 404, description = "Not Found")
), params(("id" = schema::AccountId, description = "Account ID")))]
async fn accounts_show(
    State(state): State<AppState>,
    Path(id): Path<schema::AccountId>,
) -> Result<Json<schema::Account>, AppError> {
    Ok(Json(state.repo.find_account(id.0).await?.into()))
}

// #[derive(Deserialize, IntoParams)]
// struct AssetsIndexFilter {
//     /// Filter by an owning account
//     owned_by: Option<schema::AccountId<'static>>,
//     // TODO: filter by owner/definition domain?
//     // /// Filter by the domain
//     // domain: Option<schema::DomainId<'static>>,
// }
//
// /// List assets
// #[utoipa::path(
//     get,
//     path = "/api/v1/assets",
//     params(schema::PaginationQueryParams, AssetsIndexFilter),
//     responses(
//         (status = 200, description = "OK", body = [schema::Asset])
//     )
// )]
// async fn assets_index(
//     State(state): State<AppState>,
//     Query(pagination_query): Query<schema::PaginationQueryParams>,
//     Query(filter): Query<AssetsIndexFilter>,
// ) -> Result<impl IntoResponse, AppError> {
//     let pagination = DirectPagination::from(pagination_query);
//     let predicate = if let Some(owner) = filter.owned_by {
//         AssetPredicateBox::build(|asset| asset.id.account.eq(owner.0.into_owned()))
//     } else {
//         CompoundPredicate::PASS
//     };
//     let assets = state
//         .iroha
//         .query(FindAssets)
//         .paginate(pagination)
//         .filter(predicate)
//         .all()
//         .await?;
//     Ok(Json(util_page(assets.iter(), pagination)).into_response())
// }
//
// /// Find an asset
// #[utoipa::path(get, path = "/api/v1/assets/{id}", responses(
//     (status = 200, description = "Found", body = schema::Asset),
//     (status = 404, description = "Not Found")
// ), params(("id" = schema::AssetId, description = "Asset ID")))]
// async fn assets_show(
//     State(state): State<AppState>,
//     Path(id): Path<schema::AssetId<'_>>,
// ) -> Result<impl IntoResponse, AppError> {
//     let asset = state
//         .iroha
//         .query(FindAssets)
//         .filter_with(|asset| asset.id.eq(id.0.into_owned()))
//         .one()
//         .await?
//         .ok_or(AppError::NotFound)?;
//     Ok(Json(asset.to_app_schema()).into_response())
// }
//
// #[derive(IntoParams, Deserialize)]
// struct AssetDefinitionsIndexFilter {
//     /// Filter by domain
//     domain: Option<schema::DomainId<'static>>,
// }
//
// /// List asset definitions
// #[utoipa::path(
//     get,
//     path = "/api/v1/asset-definitions",
//     params(schema::PaginationQueryParams, AssetDefinitionsIndexFilter),
//     responses(
//         (status = 200, description = "OK", body = [schema::AssetDefinition])
//     )
// )]
// async fn asset_definitions_index(
//     State(state): State<AppState>,
//     Query(pagination_query): Query<schema::PaginationQueryParams>,
//     Query(filter): Query<AssetDefinitionsIndexFilter>,
// ) -> Result<impl IntoResponse, AppError> {
//     let pagination = DirectPagination::from(pagination_query);
//     let predicate = if let Some(domain) = filter.domain {
//         AssetDefinitionPredicateBox::build(|def| def.id.domain_id.eq(domain.0.into_owned()))
//     } else {
//         CompoundPredicate::PASS
//     };
//     let items = state
//         .iroha
//         .query(FindAssetsDefinitions)
//         .paginate(pagination)
//         .filter(predicate)
//         .all()
//         .await?;
//     Ok(Json(util_page(items.iter(), pagination)).into_response())
// }
//
// /// Find an asset definition
// #[utoipa::path(get, path = "/api/v1/asset-definitions/{id}", responses(
//     (status = 200, description = "Found", body = schema::AssetDefinition),
//     (status = 404, description = "Not Found")
// ), params(("id" = schema::AssetDefinitionId, description = "Asset Definition ID")))]
// async fn asset_definitions_show(
//     State(state): State<AppState>,
//     Path(id): Path<schema::AssetDefinitionId<'_>>,
// ) -> Result<impl IntoResponse, AppError> {
//     let definition = state
//         .iroha
//         .query(FindAssetsDefinitions)
//         .filter_with(|definition| definition.id.eq(id.0.into_owned()))
//         .one()
//         .await?
//         .ok_or(AppError::NotFound)?;
//     Ok(Json(definition.to_app_schema()).into_response())
// }

pub fn router(iroha: Client, repo: Repo) -> Router {
    Router::new()
        .route("/domains", get(domains_index))
        .route("/domains/:id", get(domains_show))
        .route("/accounts", get(accounts_index))
        .route("/accounts/:id", get(accounts_show))
        // .route("/asset-definitions", get(asset_definitions_index))
        // .route("/asset-definitions/:id", get(asset_definitions_show))
        // .route("/assets", get(assets_index))
        // .route("/assets/:id", get(assets_show))
        .route("/blocks", get(blocks_index))
        .route("/blocks/:height_or_hash", get(blocks_show))
        .route("/transactions", get(transactions_index))
        .route("/transactions/:hash", get(transactions_show))
        .with_state(AppState {
            iroha: Arc::new(iroha),
            repo,
        })
}

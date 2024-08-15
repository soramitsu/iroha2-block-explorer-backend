use std::{num::NonZero, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use iroha_crypto::HashOf;
use iroha_data_model::{
    prelude::{
        FindBlockHeaderByHash, FindBlocks, FindDomains, FindTransactionByHash, FindTransactions,
    },
    query::{error::QueryExecutionFail, parameters::Pagination},
    ValidationFail,
};
use nonzero_ext::nonzero;

use crate::{
    iroha::{Client, Error as IrohaError},
    util::{DirectPagination, ReversePagination},
};
use crate::{schema, util::ReversePaginationError};

#[derive(Clone)]
struct AppState {
    client: Arc<Client>,
}

#[derive(thiserror::Error, Debug)]
enum AppError {
    #[error("failed to perform Iroha query: {0}")]
    IrohaClientError(#[from] crate::iroha::Error),
    #[error("not found")]
    NotFound,
    #[error("invalid pagination: {0}")]
    BadPage(#[from] ReversePaginationError),
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
            AppError::BadPage(x) => (StatusCode::BAD_REQUEST, format!("{x}")).into_response(),
        }
    }
}

/// List domains
#[utoipa::path(
    get,
    path = "/api/v1/domains",
    responses(
        (status = 200, description = "OK", body = schema::DomainsPage)
    ),
    params(
        schema::PaginationQueryParams
    )
)]
async fn domains_index(
    State(state): State<AppState>,
    Query(pagination_query): Query<schema::PaginationQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    let pagination = DirectPagination::from(pagination_query);
    let domains = state
        .client
        .query(FindDomains)
        .paginate(pagination)
        .all()
        .await?;
    let page = schema::Page::new(
        domains.iter().map(schema::Domain::from).collect(),
        pagination.into(),
    );
    Ok(Json(page).into_response())
}

/// Find a domain
#[utoipa::path(get, path = "/api/v1/domains/{id}", responses(
    (status = 200, description = "Domain Found", body = schema::Domain),
    (status = 404, description = "Domain Not Found")
), params(("id", description = "Domain ID", example = "genesis")))]
async fn domains_show(
    State(state): State<AppState>,
    Path(id): Path<schema::DomainId<'_>>,
) -> Result<impl IntoResponse, AppError> {
    let domain = state
        .client
        .query(FindDomains)
        .filter(|domain| domain.id.eq(id.0.into_owned()))
        .one()
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(schema::Domain::from(&domain)).into_response())
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
    Query(pagination_query): Query<schema::PaginationQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    let height = state.client.status().await?.blocks;
    let Some(height) = NonZero::new(height) else {
        return Ok(Json(schema::Page::empty(pagination_query.per_page)).into_response());
    };
    let pagination =
        ReversePagination::new(height, pagination_query.per_page, pagination_query.page)?;

    let blocks = state
        .client
        .query(FindBlocks)
        .paginate(pagination)
        .all()
        .await?;

    let page = schema::Page::new(
        blocks.iter().map(schema::Block::from).collect(),
        pagination.into(),
    );
    Ok(Json(page).into_response())
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
) -> Result<impl IntoResponse, AppError> {
    let height = match height_or_hash {
        schema::BlockHeightOrHash::Height(height) => height,
        schema::BlockHeightOrHash::Hash(hash) => state
            .client
            .query_singular(FindBlockHeaderByHash::new(HashOf::from_untyped_unchecked(
                hash,
            )))
            .await?
            .height(),
    };

    let max_height = state.client.status().await?.blocks;
    let start = if height.get() > max_height {
        return Err(AppError::NotFound);
    } else {
        max_height - height.get()
    };
    let block = state
        .client
        .query(FindBlocks)
        .paginate(Pagination::new(start, Some(nonzero!(1u64))))
        .one()
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(schema::Block::from(&block)).into_response())
}

/// List transactions
#[utoipa::path(
    get,
    path = "/api/v1/transactions",
    params(schema::PaginationQueryParams),
    responses(
        (status = 200, description = "OK", body = [schema::Transaction])
    )
)]
async fn transactions_index(
    State(state): State<AppState>,
    Query(pagination_query): Query<schema::PaginationQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    let pagination = DirectPagination::from(pagination_query);
    let items = state
        .client
        .query(FindTransactions)
        .paginate(pagination)
        .all()
        .await?;
    let page = schema::Page::new(
        items
            .iter()
            .map(schema::TransactionWithHash::from)
            .collect(),
        pagination.into(),
    );
    Ok(Json(page).into_response())
}

/// Find a transaction by its hash
#[utoipa::path(get, path = "/api/v1/transactions/{hash}", params(
    ("hash", description = "Hash of the transaction", example = "9FC55BD948D0CDE0838F6D86FA069A258F033156EE9ACEF5A5018BC9589473F3")
), responses(
    (status = 200, description = "Transaction Found", body = schema::TransactionWithHash),
    (status = 404, description = "Transaction Not Found")
))]
async fn transactions_show(
    State(state): State<AppState>,
    Path(hash): Path<iroha_crypto::Hash>,
) -> Result<impl IntoResponse, AppError> {
    let item = state
        .client
        .query_singular(FindTransactionByHash::new(HashOf::from_untyped_unchecked(
            hash,
        )))
        .await?;
    Ok(Json(schema::TransactionWithHash::from(&item)).into_response())
}

pub fn router(client: Client) -> Router {
    Router::new()
        .route("/domains", get(domains_index))
        .route("/domains/:id", get(domains_show))
        .route("/blocks", get(blocks_index))
        .route("/blocks/:height_or_hash", get(blocks_show))
        .route("/transactions", get(transactions_index))
        .route("/transactions/:hash", get(transactions_show))
        .with_state(AppState {
            client: Arc::new(client),
        })
}

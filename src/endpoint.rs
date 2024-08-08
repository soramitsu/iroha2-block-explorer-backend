use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use iroha_data_model::{
    domain::DomainId, prelude::FindDomains, query::error::QueryExecutionFail, ValidationFail,
};

use crate::iroha::{Client, Error as IrohaError};

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
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::IrohaClientError(IrohaError::QueryValidationFail {
                reason: ValidationFail::QueryFailed(QueryExecutionFail::Find(error)),
            }) => (
                StatusCode::NOT_FOUND,
                format!("Iroha couldn't find requested resource: {error}"),
            )
                .into_response(),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not found").into_response(),
            err => {
                tracing::error!(%err, "internal server error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong").into_response()
            }
        }
    }
}

pub fn router(client: Client) -> Router {
    Router::new()
        .route("/domains", get(domains_index))
        .route("/domains/:id", get(domains_show))
        .with_state(AppState {
            client: Arc::new(client),
        })
}

async fn domains_index(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let domains = state.client.query(FindDomains).all().await?;
    Ok(Json(domains))
}

async fn domains_show(
    State(state): State<AppState>,
    Path(id): Path<DomainId>,
) -> Result<impl IntoResponse, AppError> {
    let domain = state
        .client
        .query(FindDomains)
        .filter(|domain| domain.id.eq(id))
        .one()
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(domain))
}

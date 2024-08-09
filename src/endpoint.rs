use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use iroha_data_model::{prelude::FindDomains, query::error::QueryExecutionFail, ValidationFail};

use crate::iroha::{Client, Error as IrohaError};
use crate::schema;

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

/// List all domains
#[utoipa::path(get, path = "/api/v1/domains", responses(
    (status = 200, description = "OK", body = [schema::Domain])
))]
async fn domains_index(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let domains = state.client.query(FindDomains).all().await?;
    let dto: Vec<_> = domains.iter().map(schema::Domain::from).collect();
    Ok(Json(dto).into_response())
}

/// Show a certain domain
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

pub fn router(client: Client) -> Router {
    Router::new()
        .route("/domains", get(domains_index))
        .route("/domains/:id", get(domains_show))
        .with_state(AppState {
            client: Arc::new(client),
        })
}

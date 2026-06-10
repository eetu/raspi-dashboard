use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    /// An upstream (gatus / beszel) returned an error or unparseable response.
    #[error("upstream: {0}")]
    Upstream(String),
    /// A dependency is configured but not currently reachable / not yet
    /// authenticated — retryable. e.g. beszel creds unset, or the trivy scan
    /// file doesn't exist yet.
    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Upstream(_) => StatusCode::BAD_GATEWAY,
            AppError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = Json(json!({
            "error": status.as_u16(),
            "detail": self.to_string(),
        }));
        if status.is_server_error() {
            tracing::error!(?self, "request failed");
        } else if matches!(
            self,
            AppError::Upstream(_) | AppError::ServiceUnavailable(_)
        ) {
            tracing::warn!(?self, "request degraded");
        }
        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Upstream(e.to_string())
    }
}

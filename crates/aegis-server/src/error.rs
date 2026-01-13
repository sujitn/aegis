//! API error types.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;

/// API errors.
#[derive(Debug, Error)]
pub enum ApiError {
    /// Authentication required.
    #[error("authentication required")]
    Unauthorized,

    /// Invalid credentials.
    #[error("invalid credentials")]
    InvalidCredentials,

    /// Session expired.
    #[error("session expired")]
    SessionExpired,

    /// Resource not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Bad request.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// Internal server error.
    #[error("internal error: {0}")]
    Internal(String),

    /// Storage error.
    #[error("storage error: {0}")]
    Storage(#[from] aegis_storage::StorageError),

    /// Auth error.
    #[error("auth error: {0}")]
    Auth(#[from] aegis_core::auth::AuthError),
}

/// Error response body.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "invalid_credentials"),
            ApiError::SessionExpired => (StatusCode::UNAUTHORIZED, "session_expired"),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
            ApiError::Storage(_) => (StatusCode::INTERNAL_SERVER_ERROR, "storage_error"),
            ApiError::Auth(_) => (StatusCode::INTERNAL_SERVER_ERROR, "auth_error"),
        };

        let body = ErrorResponse {
            error: self.to_string(),
            code: code.to_string(),
        };

        (status, axum::Json(body)).into_response()
    }
}

/// Result type for API operations.
pub type Result<T> = std::result::Result<T, ApiError>;

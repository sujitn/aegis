//! Error types for the UI crate.

use thiserror::Error;

/// UI-specific errors.
#[derive(Debug, Error)]
pub enum UiError {
    /// Storage error.
    #[error("storage error: {0}")]
    Storage(#[from] aegis_storage::StorageError),

    /// Authentication error.
    #[error("authentication error: {0}")]
    Auth(#[from] aegis_core::auth::AuthError),

    /// Session expired or not authenticated.
    #[error("session expired")]
    SessionExpired,

    /// Invalid input.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// CSV export error.
    #[error("export error: {0}")]
    Export(String),

    /// CSV error.
    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for UI operations.
pub type Result<T> = std::result::Result<T, UiError>;

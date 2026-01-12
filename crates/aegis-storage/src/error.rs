//! Storage error types.

use thiserror::Error;

/// Errors that can occur in storage operations.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Database error from rusqlite.
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// IO error (e.g., creating directories).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Record not found.
    #[error("Record not found: {0}")]
    NotFound(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Migration error.
    #[error("Migration error: {0}")]
    Migration(String),
}

/// Result type for storage operations.
pub type Result<T> = std::result::Result<T, StorageError>;

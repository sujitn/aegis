//! Error types for the proxy.

use thiserror::Error;

/// Proxy error type.
#[derive(Debug, Error)]
pub enum ProxyError {
    /// CA certificate error.
    #[error("CA error: {0}")]
    Ca(#[from] CaManagerError),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// TLS error.
    #[error("TLS error: {0}")]
    Tls(String),

    /// HTTP error.
    #[error("HTTP error: {0}")]
    Http(String),

    /// JSON parsing error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Storage error.
    #[error("Storage error: {0}")]
    Storage(String),

    /// Classification error.
    #[error("Classification error: {0}")]
    Classification(String),

    /// Proxy server error.
    #[error("Proxy error: {0}")]
    Proxy(String),
}

/// CA manager error type.
#[derive(Debug, Error)]
pub enum CaManagerError {
    /// Failed to generate CA certificate.
    #[error("Failed to generate CA: {0}")]
    Generation(String),

    /// Failed to read CA certificate.
    #[error("Failed to read CA: {0}")]
    Read(#[from] std::io::Error),

    /// Failed to parse CA certificate.
    #[error("Failed to parse CA: {0}")]
    Parse(String),

    /// Failed to write CA certificate.
    #[error("Failed to write CA: {0}")]
    Write(String),
}

/// Result type for proxy operations.
pub type Result<T> = std::result::Result<T, ProxyError>;

//! Tray error types.

use thiserror::Error;

/// Errors that can occur during tray operations.
#[derive(Debug, Error)]
pub enum TrayError {
    /// Failed to create tray icon.
    #[error("failed to create tray icon: {0}")]
    IconCreation(String),

    /// Failed to load icon image.
    #[error("failed to load icon: {0}")]
    IconLoad(String),

    /// Failed to create menu.
    #[error("failed to create menu: {0}")]
    MenuCreation(String),

    /// Failed to update tray.
    #[error("failed to update tray: {0}")]
    Update(String),

    /// Tray not initialized.
    #[error("tray not initialized")]
    NotInitialized,

    /// Platform not supported.
    #[error("platform not supported: {0}")]
    PlatformNotSupported(String),
}

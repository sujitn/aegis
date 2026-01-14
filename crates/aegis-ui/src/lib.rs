//! Aegis UI - Parent Dashboard GUI.
//!
//! This crate provides the parent dashboard for the Aegis platform.
//! It includes:
//!
//! - Password-protected access with session timeout
//! - Dashboard with summary statistics and quick actions
//! - Profile management (create, edit, delete)
//! - Rules configuration (time rules, content rules)
//! - Activity logs with filtering and export
//! - Settings (password change, mode selection)
//!
//! # Usage
//!
//! ```no_run
//! use aegis_ui::{DashboardApp, run_dashboard};
//! use aegis_storage::Database;
//!
//! // Create database
//! let db = Database::new().expect("Failed to open database");
//!
//! // Run the dashboard
//! run_dashboard(db).expect("Failed to run dashboard");
//! ```

mod app;
pub mod error;
pub mod state;
pub mod views;

pub use app::DashboardApp;
pub use error::{Result, UiError};
pub use state::{AppState, InterceptionMode, ProtectionStatus, View};

/// Runs the parent dashboard application.
///
/// This is the main entry point for the GUI application.
pub fn run_dashboard(db: aegis_storage::Database) -> Result<()> {
    let app = DashboardApp::new(db);
    let options = DashboardApp::window_options();

    eframe::run_native(
        "Aegis Dashboard",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .map_err(|e| UiError::InvalidInput(e.to_string()))
}

/// Placeholder for backwards compatibility with previous API.
pub mod settings {
    // Settings UI module (backwards compatibility).

    use crate::DashboardApp;

    /// Placeholder type for settings UI functionality.
    pub struct SettingsUi {
        #[allow(dead_code)]
        app: Option<DashboardApp>,
    }

    impl SettingsUi {
        /// Creates a new settings UI instance.
        pub fn new() -> Self {
            Self { app: None }
        }

        /// Creates with an existing app instance.
        pub fn with_app(app: DashboardApp) -> Self {
            Self { app: Some(app) }
        }
    }

    impl Default for SettingsUi {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_ui_can_be_created() {
        let _ui = settings::SettingsUi::new();
    }

    #[test]
    fn test_app_state_creation() {
        let db = aegis_storage::Database::in_memory().unwrap();
        let state = AppState::new(db);
        // First setup starts with Setup view
        assert_eq!(state.view, View::Setup);
    }

    #[test]
    fn test_protection_status() {
        assert_eq!(ProtectionStatus::Active.as_str(), "Active");
        assert_eq!(ProtectionStatus::Paused.as_str(), "Paused");
        assert_eq!(ProtectionStatus::Disabled.as_str(), "Disabled");
    }

    #[test]
    fn test_interception_mode() {
        assert_eq!(InterceptionMode::Proxy.as_str(), "Proxy");
    }
}

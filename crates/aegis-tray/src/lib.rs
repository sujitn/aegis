//! Aegis Tray - System tray integration.
//!
//! This crate provides system tray functionality for the Aegis platform,
//! including status indicators and menu actions.

mod error;
mod icon;
mod menu;
mod status;
mod tray;

pub use error::TrayError;
pub use icon::TrayIcon;
pub use menu::{MenuAction, TrayMenu};
pub use status::TrayStatus;
pub use tray::{SystemTray, TrayConfig, TrayEvent};

/// Result type for tray operations.
pub type Result<T> = std::result::Result<T, TrayError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_status_display() {
        assert_eq!(TrayStatus::Protected.as_str(), "Protected");
        assert_eq!(TrayStatus::Paused.as_str(), "Paused");
        assert_eq!(TrayStatus::Error.as_str(), "Error");
    }

    #[test]
    fn tray_status_tooltip() {
        assert_eq!(TrayStatus::Protected.tooltip(), "Aegis - Protection Active");
        assert_eq!(TrayStatus::Paused.tooltip(), "Aegis - Protection Paused");
        assert_eq!(TrayStatus::Error.tooltip(), "Aegis - Service Error");
    }

    #[test]
    fn menu_action_from_id() {
        assert_eq!(
            MenuAction::from_id("dashboard"),
            Some(MenuAction::Dashboard)
        );
        assert_eq!(MenuAction::from_id("settings"), Some(MenuAction::Settings));
        assert_eq!(MenuAction::from_id("logs"), Some(MenuAction::Logs));
        assert_eq!(MenuAction::from_id("pause"), Some(MenuAction::Pause));
        assert_eq!(MenuAction::from_id("resume"), Some(MenuAction::Resume));
        assert_eq!(MenuAction::from_id("quit"), Some(MenuAction::Quit));
        assert_eq!(MenuAction::from_id("unknown"), None);
    }

    #[test]
    fn tray_config_default() {
        let config = TrayConfig::default();
        assert_eq!(config.app_name, "Aegis");
        assert_eq!(config.initial_status, TrayStatus::Protected);
    }

    #[test]
    fn tray_config_builder() {
        let config = TrayConfig::new()
            .with_app_name("Test App")
            .with_initial_status(TrayStatus::Paused);

        assert_eq!(config.app_name, "Test App");
        assert_eq!(config.initial_status, TrayStatus::Paused);
    }

    #[test]
    fn tray_event_variants() {
        let event = TrayEvent::MenuAction(MenuAction::Dashboard);
        assert!(matches!(
            event,
            TrayEvent::MenuAction(MenuAction::Dashboard)
        ));

        let event = TrayEvent::DoubleClick;
        assert!(matches!(event, TrayEvent::DoubleClick));

        let event = TrayEvent::StatusChanged(TrayStatus::Error);
        assert!(matches!(event, TrayEvent::StatusChanged(TrayStatus::Error)));
    }
}

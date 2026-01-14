//! Main system tray implementation.

use crate::{icon::TrayIcon, menu::TrayMenu, status::TrayStatus, MenuAction, TrayError};
use muda::MenuEvent;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, Sender},
    Arc,
};
use tray_icon::{TrayIcon as TrayIconHandle, TrayIconBuilder, TrayIconEvent};

/// Events emitted by the system tray.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayEvent {
    /// A menu action was triggered.
    MenuAction(MenuAction),

    /// The tray icon was double-clicked.
    DoubleClick,

    /// The protection status changed.
    StatusChanged(TrayStatus),
}

/// Configuration for the system tray.
#[derive(Debug, Clone)]
pub struct TrayConfig {
    /// Application name shown in tooltip.
    pub app_name: &'static str,

    /// Initial protection status.
    pub initial_status: TrayStatus,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            app_name: "Aegis",
            initial_status: TrayStatus::Protected,
        }
    }
}

impl TrayConfig {
    /// Creates a new configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the application name.
    pub fn with_app_name(mut self, name: &'static str) -> Self {
        self.app_name = name;
        self
    }

    /// Sets the initial status.
    pub fn with_initial_status(mut self, status: TrayStatus) -> Self {
        self.initial_status = status;
        self
    }
}

/// System tray manager.
///
/// Provides system tray functionality with status indicators and menu actions.
/// The tray runs in the background and emits events through a channel.
pub struct SystemTray {
    /// The tray icon handle.
    tray_icon: Option<TrayIconHandle>,

    /// The tray menu.
    menu: Option<TrayMenu>,

    /// Current protection status.
    status: TrayStatus,

    /// Event sender for tray events.
    event_tx: Sender<TrayEvent>,

    /// Whether the tray is running.
    running: Arc<AtomicBool>,

    /// Configuration (for future use).
    #[allow(dead_code)]
    config: TrayConfig,
}

impl SystemTray {
    /// Creates a new system tray with default configuration.
    pub fn new() -> crate::Result<(Self, Receiver<TrayEvent>)> {
        Self::with_config(TrayConfig::default())
    }

    /// Creates a new system tray with the given configuration.
    pub fn with_config(config: TrayConfig) -> crate::Result<(Self, Receiver<TrayEvent>)> {
        let (event_tx, event_rx) = mpsc::channel();

        let tray = Self {
            tray_icon: None,
            menu: None,
            status: config.initial_status,
            event_tx,
            running: Arc::new(AtomicBool::new(false)),
            config,
        };

        Ok((tray, event_rx))
    }

    /// Initializes and shows the system tray.
    ///
    /// This must be called from the main thread on some platforms.
    pub fn init(&mut self) -> crate::Result<()> {
        // Create the menu
        let menu = TrayMenu::new(self.status)?;

        // Create the icon
        let icon = TrayIcon::for_status(self.status)?;

        // Build the tray icon
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu.menu().clone()))
            .with_tooltip(self.status.tooltip())
            .with_icon(icon)
            .build()
            .map_err(|e| TrayError::IconCreation(e.to_string()))?;

        self.tray_icon = Some(tray_icon);
        self.menu = Some(menu);
        self.running.store(true, Ordering::SeqCst);

        tracing::info!(status = %self.status, "System tray initialized");

        Ok(())
    }

    /// Polls for tray events.
    ///
    /// This should be called regularly from an event loop to process events.
    /// Returns any events that occurred.
    pub fn poll_events(&self) -> Vec<TrayEvent> {
        let mut events = Vec::new();

        // Process menu events
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            let id_str = event.id.0.as_str();
            if let Some(action) = MenuAction::from_id(id_str) {
                events.push(TrayEvent::MenuAction(action));
                let _ = self.event_tx.send(TrayEvent::MenuAction(action));
            }
        }

        // Process tray icon events (double-click)
        // Note: Double-click detection depends on platform-specific behavior
        // Some platforms use dedicated double-click events, others need timing logic
        if let Ok(_event) = TrayIconEvent::receiver().try_recv() {
            // Currently we handle menu events primarily.
            // Double-click opens settings, which is the same as the Settings menu action.
            // For now, users can use the menu to access settings.
        }

        events
    }

    /// Updates the protection status.
    pub fn set_status(&mut self, status: TrayStatus) -> crate::Result<()> {
        if self.status == status {
            return Ok(());
        }

        self.status = status;

        // Update the icon
        if let Some(tray_icon) = &self.tray_icon {
            let icon = TrayIcon::for_status(status)?;
            tray_icon
                .set_icon(Some(icon))
                .map_err(|e| TrayError::Update(e.to_string()))?;
            tray_icon
                .set_tooltip(Some(status.tooltip()))
                .map_err(|e| TrayError::Update(e.to_string()))?;
        }

        // Update the menu
        if let Some(menu) = &mut self.menu {
            menu.update_status(status);
        }

        // Emit status change event
        let _ = self.event_tx.send(TrayEvent::StatusChanged(status));

        tracing::info!(status = %status, "Tray status updated");

        Ok(())
    }

    /// Returns the current protection status.
    pub fn status(&self) -> TrayStatus {
        self.status
    }

    /// Returns whether the tray is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Shuts down the system tray.
    pub fn shutdown(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.tray_icon = None;
        self.menu = None;
        tracing::info!("System tray shutdown");
    }
}

impl Drop for SystemTray {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        let config = TrayConfig::default();
        assert_eq!(config.app_name, "Aegis");
        assert_eq!(config.initial_status, TrayStatus::Protected);
    }

    #[test]
    fn config_builder_chain() {
        let config = TrayConfig::new()
            .with_app_name("Test")
            .with_initial_status(TrayStatus::Error);

        assert_eq!(config.app_name, "Test");
        assert_eq!(config.initial_status, TrayStatus::Error);
    }

    #[test]
    fn tray_event_equality() {
        assert_eq!(
            TrayEvent::MenuAction(MenuAction::Dashboard),
            TrayEvent::MenuAction(MenuAction::Dashboard)
        );
        assert_ne!(
            TrayEvent::MenuAction(MenuAction::Dashboard),
            TrayEvent::MenuAction(MenuAction::Quit)
        );
        assert_eq!(TrayEvent::DoubleClick, TrayEvent::DoubleClick);
        assert_eq!(
            TrayEvent::StatusChanged(TrayStatus::Protected),
            TrayEvent::StatusChanged(TrayStatus::Protected)
        );
    }

    #[test]
    fn can_create_tray_without_init() {
        // Creating a tray should work without GUI
        let result = SystemTray::new();
        assert!(result.is_ok());

        let (tray, _rx) = result.unwrap();
        assert_eq!(tray.status(), TrayStatus::Protected);
        assert!(!tray.is_running()); // Not running until init()
    }

    #[test]
    fn tray_with_custom_config() {
        let config = TrayConfig::new().with_initial_status(TrayStatus::Paused);

        let (tray, _rx) = SystemTray::with_config(config).unwrap();
        assert_eq!(tray.status(), TrayStatus::Paused);
    }
}

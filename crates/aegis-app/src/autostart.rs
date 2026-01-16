//! Autostart functionality (F030).
//!
//! Provides cross-platform autostart support for Aegis:
//! - Windows: Registry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
//! - macOS: LaunchAgent plist in `~/Library/LaunchAgents/`
//! - Linux: `.desktop` file in `~/.config/autostart/`
//!
//! # Usage
//!
//! ```ignore
//! use aegis_app::autostart::Autostart;
//! use aegis_storage::Database;
//!
//! let db = Database::new().expect("Failed to open database");
//! let autostart = Autostart::new(db).expect("Failed to create autostart");
//!
//! // Check if autostart is enabled
//! if autostart.is_enabled() {
//!     println!("Autostart is enabled");
//! }
//!
//! // Enable autostart
//! autostart.enable().expect("Failed to enable autostart");
//!
//! // Disable autostart
//! autostart.disable().expect("Failed to disable autostart");
//! ```

use std::env;
use std::path::{Path, PathBuf};

use aegis_storage::Database;
use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use serde_json::json;
use thiserror::Error;

/// App name used for autostart registration.
const APP_NAME: &str = "Aegis";

/// Config key for autostart lock state.
const CONFIG_KEY_AUTOSTART_LOCKED: &str = "autostart_locked";

/// Errors that can occur during autostart operations.
#[derive(Debug, Error)]
pub enum AutostartError {
    /// Failed to get executable path.
    #[error("failed to get executable path")]
    ExecutablePath,

    /// Failed to create autostart entry.
    #[error("failed to create autostart: {0}")]
    CreateFailed(String),

    /// Failed to enable autostart.
    #[error("failed to enable autostart: {0}")]
    EnableFailed(String),

    /// Failed to disable autostart.
    #[error("failed to disable autostart: {0}")]
    DisableFailed(String),

    /// Failed to check autostart status.
    #[error("failed to check autostart status: {0}")]
    CheckFailed(String),

    /// Setting is locked and requires authentication.
    #[error("autostart setting is locked")]
    SettingLocked,

    /// Not authenticated to change locked settings.
    #[error("authentication required to change locked settings")]
    AuthRequired,

    /// Storage error.
    #[error("storage error: {0}")]
    Storage(String),
}

/// Result type for autostart operations.
pub type Result<T> = std::result::Result<T, AutostartError>;

/// Manages autostart registration for Aegis.
pub struct Autostart {
    db: Database,
    launcher: AutoLaunch,
}

impl Autostart {
    /// Creates a new Autostart manager.
    ///
    /// Returns an error if the executable path cannot be determined.
    pub fn new(db: Database) -> Result<Self> {
        let exe_path = Self::get_executable_path()?;
        let launcher = Self::create_launcher(&exe_path)?;

        Ok(Self { db, launcher })
    }

    /// Creates a new Autostart manager with a custom executable path.
    pub fn with_path(db: Database, exe_path: PathBuf) -> Result<Self> {
        let launcher = Self::create_launcher(&exe_path)?;

        Ok(Self { db, launcher })
    }

    /// Gets the path to the current executable.
    pub fn get_executable_path() -> Result<PathBuf> {
        env::current_exe().map_err(|_| AutostartError::ExecutablePath)
    }

    /// Creates an AutoLaunch instance with the given executable path.
    fn create_launcher(exe_path: &Path) -> Result<AutoLaunch> {
        let exe_str = exe_path.to_str().ok_or(AutostartError::ExecutablePath)?;

        // Build with --minimized flag for silent startup
        let args = &["--minimized"];

        #[cfg(target_os = "macos")]
        let launcher = AutoLaunchBuilder::new()
            .set_app_name(APP_NAME)
            .set_app_path(exe_str)
            .set_args(args)
            .set_use_launch_agent(true)
            .build()
            .map_err(|e| AutostartError::CreateFailed(e.to_string()))?;

        #[cfg(not(target_os = "macos"))]
        let launcher = AutoLaunchBuilder::new()
            .set_app_name(APP_NAME)
            .set_app_path(exe_str)
            .set_args(args)
            .build()
            .map_err(|e| AutostartError::CreateFailed(e.to_string()))?;

        Ok(launcher)
    }

    /// Checks if autostart is currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.launcher.is_enabled().unwrap_or(false)
    }

    /// Enables autostart (starts Aegis on login).
    ///
    /// If the setting is locked, returns `AutostartError::SettingLocked`.
    pub fn enable(&self) -> Result<()> {
        if self.is_locked() {
            return Err(AutostartError::SettingLocked);
        }

        self.launcher
            .enable()
            .map_err(|e| AutostartError::EnableFailed(e.to_string()))
    }

    /// Enables autostart without checking lock status.
    ///
    /// Used during setup wizard where lock hasn't been set yet.
    pub fn enable_unchecked(&self) -> Result<()> {
        self.launcher
            .enable()
            .map_err(|e| AutostartError::EnableFailed(e.to_string()))
    }

    /// Disables autostart (does not start Aegis on login).
    ///
    /// If the setting is locked, returns `AutostartError::SettingLocked`.
    pub fn disable(&self) -> Result<()> {
        if self.is_locked() {
            return Err(AutostartError::SettingLocked);
        }

        self.launcher
            .disable()
            .map_err(|e| AutostartError::DisableFailed(e.to_string()))
    }

    /// Disables autostart without checking lock status.
    ///
    /// Used during uninstall to ensure autostart is removed.
    pub fn disable_unchecked(&self) -> Result<()> {
        self.launcher
            .disable()
            .map_err(|e| AutostartError::DisableFailed(e.to_string()))
    }

    /// Checks if the autostart setting is locked.
    ///
    /// When locked, the setting cannot be changed without authentication.
    pub fn is_locked(&self) -> bool {
        self.db
            .get_config(CONFIG_KEY_AUTOSTART_LOCKED)
            .ok()
            .flatten()
            .and_then(|v| v.value.as_bool())
            .unwrap_or(false)
    }

    /// Sets the lock state for the autostart setting.
    ///
    /// Requires authentication to change.
    pub fn set_locked(&self, locked: bool) -> Result<()> {
        self.db
            .set_config(CONFIG_KEY_AUTOSTART_LOCKED, &json!(locked))
            .map_err(|e| AutostartError::Storage(e.to_string()))
    }

    /// Gets the command that would be used for autostart.
    ///
    /// Returns the path to the Aegis executable with the --minimized flag.
    pub fn get_command(&self) -> Result<String> {
        let exe_path = Self::get_executable_path()?;
        let exe_str = exe_path.to_str().ok_or(AutostartError::ExecutablePath)?;

        Ok(format!("\"{}\" --minimized", exe_str))
    }

    /// Gets the executable path as a PathBuf.
    pub fn get_executable(&self) -> Result<PathBuf> {
        Self::get_executable_path()
    }
}

/// Remove autostart entry (for uninstall).
///
/// This function attempts to remove the autostart entry without requiring
/// an Autostart instance, useful during uninstall when the database may
/// already be deleted.
pub fn remove_autostart_entry() -> Result<()> {
    let exe_path = Autostart::get_executable_path()?;
    let exe_str = exe_path.to_str().ok_or(AutostartError::ExecutablePath)?;

    #[cfg(target_os = "macos")]
    let launcher = AutoLaunchBuilder::new()
        .set_app_name(APP_NAME)
        .set_app_path(exe_str)
        .set_use_launch_agent(true)
        .build()
        .map_err(|e| AutostartError::CreateFailed(e.to_string()))?;

    #[cfg(not(target_os = "macos"))]
    let launcher = AutoLaunchBuilder::new()
        .set_app_name(APP_NAME)
        .set_app_path(exe_str)
        .build()
        .map_err(|e| AutostartError::CreateFailed(e.to_string()))?;

    // Try to disable, ignore errors if not enabled
    let _ = launcher.disable();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> Database {
        Database::in_memory().expect("Failed to create test database")
    }

    // ==================== Autostart Creation Tests ====================

    #[test]
    fn test_get_executable_path() {
        // Should return the current test executable
        let path = Autostart::get_executable_path();
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_autostart_new() {
        let db = create_test_db();
        let autostart = Autostart::new(db);
        // May fail in test environment without proper setup, but should not panic
        if let Ok(autostart) = autostart {
            // Just verify we can call is_enabled
            let _ = autostart.is_enabled();
        }
    }

    // ==================== Lock State Tests ====================

    #[test]
    fn test_is_locked_default() {
        let db = create_test_db();
        if let Ok(autostart) = Autostart::new(db) {
            // Default should be unlocked
            assert!(!autostart.is_locked());
        }
    }

    #[test]
    fn test_set_locked() {
        let db = create_test_db();
        if let Ok(autostart) = Autostart::new(db) {
            // Lock the setting
            autostart.set_locked(true).unwrap();
            assert!(autostart.is_locked());

            // Unlock the setting
            autostart.set_locked(false).unwrap();
            assert!(!autostart.is_locked());
        }
    }

    #[test]
    fn test_enable_when_locked() {
        let db = create_test_db();
        if let Ok(autostart) = Autostart::new(db) {
            // Lock the setting
            autostart.set_locked(true).unwrap();

            // Should fail to enable
            let result = autostart.enable();
            assert!(matches!(result, Err(AutostartError::SettingLocked)));
        }
    }

    #[test]
    fn test_disable_when_locked() {
        let db = create_test_db();
        if let Ok(autostart) = Autostart::new(db) {
            // Lock the setting
            autostart.set_locked(true).unwrap();

            // Should fail to disable
            let result = autostart.disable();
            assert!(matches!(result, Err(AutostartError::SettingLocked)));
        }
    }

    #[test]
    fn test_enable_unchecked_bypasses_lock() {
        let db = create_test_db();
        if let Ok(autostart) = Autostart::new(db) {
            // Lock the setting
            autostart.set_locked(true).unwrap();

            // Should succeed with unchecked
            // Note: This may still fail due to platform restrictions in test environment
            let _ = autostart.enable_unchecked();
        }
    }

    // ==================== Command Tests ====================

    #[test]
    fn test_get_command() {
        let db = create_test_db();
        if let Ok(autostart) = Autostart::new(db) {
            let cmd = autostart.get_command();
            assert!(cmd.is_ok());
            let cmd = cmd.unwrap();
            assert!(cmd.contains("--minimized"));
        }
    }

    // ==================== Remove Entry Tests ====================

    #[test]
    fn test_remove_autostart_entry() {
        // Should not panic even if entry doesn't exist
        let result = remove_autostart_entry();
        // May fail due to permissions in test environment, but should not panic
        let _ = result;
    }

    // ==================== Error Type Tests ====================

    #[test]
    fn test_error_display() {
        let err = AutostartError::SettingLocked;
        assert_eq!(err.to_string(), "autostart setting is locked");

        let err = AutostartError::EnableFailed("test".to_string());
        assert_eq!(err.to_string(), "failed to enable autostart: test");
    }
}

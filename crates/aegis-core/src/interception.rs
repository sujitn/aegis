//! Interception mode module (F017).
//!
//! Provides interception mode switching between browser extension and MITM proxy.
//!
//! ## Modes
//!
//! - **Extension**: Browser-only interception via Chrome extension (simpler setup)
//! - **Proxy**: System-wide interception via MITM proxy (full protection)
//!
//! ## Usage
//!
//! ```
//! use aegis_core::interception::{InterceptionManager, InterceptionMode};
//! use aegis_core::auth::AuthManager;
//!
//! let auth = AuthManager::new();
//! let manager = InterceptionManager::new();
//!
//! // Check current mode (default is Extension)
//! assert_eq!(manager.mode(), InterceptionMode::Extension);
//!
//! // Change mode (requires valid session)
//! let session = auth.create_session();
//! manager.set_mode(InterceptionMode::Proxy, &session, &auth).unwrap();
//!
//! // Check new mode
//! assert_eq!(manager.mode(), InterceptionMode::Proxy);
//! ```

use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::auth::{AuthManager, SessionToken};

/// Interception mode errors.
#[derive(Debug, Error)]
pub enum InterceptionError {
    /// Authentication required for this operation.
    #[error("authentication required to change interception mode")]
    AuthRequired,

    /// Session is invalid or expired.
    #[error("session is invalid or expired")]
    SessionInvalid,

    /// Cannot switch to same mode.
    #[error("already in {0} mode")]
    AlreadyInMode(String),
}

/// Result type for interception operations.
pub type Result<T> = std::result::Result<T, InterceptionError>;

/// Interception mode for LLM traffic filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InterceptionMode {
    /// Browser extension mode - intercepts only browser traffic to LLM sites.
    /// Simpler setup, no CA certificate required.
    #[default]
    Extension,

    /// MITM proxy mode - intercepts all app traffic to LLM APIs.
    /// Full coverage, requires CA certificate installation.
    Proxy,
}

impl InterceptionMode {
    /// Returns the mode as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Extension => "extension",
            Self::Proxy => "proxy",
        }
    }

    /// Returns a human-readable name for the mode.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Extension => "Browser Extension",
            Self::Proxy => "System Proxy",
        }
    }

    /// Returns a description of what this mode does.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Extension => "Intercepts browser traffic to AI chat sites only",
            Self::Proxy => "Intercepts all app traffic to AI services",
        }
    }

    /// Returns setup requirements for this mode.
    pub fn setup_info(&self) -> &'static str {
        match self {
            Self::Extension => "Requires browser extension installation",
            Self::Proxy => "Requires CA certificate installation in browser/system",
        }
    }

    /// Returns the coverage level.
    pub fn coverage(&self) -> &'static str {
        match self {
            Self::Extension => "Browser only",
            Self::Proxy => "All applications",
        }
    }

    /// Returns true if this mode requires CA certificate installation.
    pub fn requires_ca_cert(&self) -> bool {
        matches!(self, Self::Proxy)
    }

    /// Returns all available modes.
    pub fn all() -> &'static [InterceptionMode] {
        &[Self::Extension, Self::Proxy]
    }
}

impl std::fmt::Display for InterceptionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Events emitted when interception mode changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterceptionEvent {
    /// Mode changed from one to another.
    ModeChanged {
        /// Previous mode.
        from: InterceptionMode,
        /// New mode.
        to: InterceptionMode,
    },
}

/// Internal state data for the interception manager.
#[derive(Debug)]
struct InterceptionData {
    /// Current interception mode.
    mode: InterceptionMode,
}

impl Default for InterceptionData {
    fn default() -> Self {
        Self {
            mode: InterceptionMode::Extension,
        }
    }
}

/// Manages interception mode with authentication-guarded switching.
///
/// Thread-safe and clonable for use across async contexts.
#[derive(Debug, Clone, Default)]
pub struct InterceptionManager {
    data: Arc<RwLock<InterceptionData>>,
}

impl InterceptionManager {
    /// Creates a new interception manager with default mode (Extension).
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(InterceptionData::default())),
        }
    }

    /// Creates a new interception manager with a specific mode.
    pub fn with_mode(mode: InterceptionMode) -> Self {
        Self {
            data: Arc::new(RwLock::new(InterceptionData { mode })),
        }
    }

    /// Returns the current interception mode.
    pub fn mode(&self) -> InterceptionMode {
        self.data.read().unwrap().mode
    }

    /// Returns true if currently in extension mode.
    pub fn is_extension_mode(&self) -> bool {
        self.mode() == InterceptionMode::Extension
    }

    /// Returns true if currently in proxy mode.
    pub fn is_proxy_mode(&self) -> bool {
        self.mode() == InterceptionMode::Proxy
    }

    /// Changes the interception mode.
    ///
    /// Requires a valid authenticated session.
    pub fn set_mode(
        &self,
        mode: InterceptionMode,
        session: &SessionToken,
        auth: &AuthManager,
    ) -> Result<InterceptionEvent> {
        // Validate session
        if !auth.validate_session(session) {
            return Err(InterceptionError::SessionInvalid);
        }

        let mut data = self.data.write().unwrap();
        let from = data.mode;

        // Check if already in this mode
        if from == mode {
            return Err(InterceptionError::AlreadyInMode(mode.to_string()));
        }

        // Set new mode
        data.mode = mode;

        Ok(InterceptionEvent::ModeChanged { from, to: mode })
    }

    /// Sets the mode directly without authentication.
    ///
    /// Use only when restoring state from persistent storage.
    pub fn restore_mode(&self, mode: InterceptionMode) {
        let mut data = self.data.write().unwrap();
        data.mode = mode;
    }

    /// Gets mode info for display.
    pub fn mode_info(&self) -> ModeInfo {
        let mode = self.mode();
        ModeInfo {
            mode,
            name: mode.name(),
            description: mode.description(),
            setup_info: mode.setup_info(),
            coverage: mode.coverage(),
            requires_ca_cert: mode.requires_ca_cert(),
        }
    }
}

/// Information about the current interception mode for UI display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeInfo {
    /// The current mode.
    pub mode: InterceptionMode,
    /// Human-readable name.
    pub name: &'static str,
    /// Description of what the mode does.
    pub description: &'static str,
    /// Setup requirements.
    pub setup_info: &'static str,
    /// Coverage level.
    pub coverage: &'static str,
    /// Whether CA certificate is required.
    pub requires_ca_cert: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== InterceptionMode Tests ====================

    #[test]
    fn test_default_mode_is_extension() {
        assert_eq!(InterceptionMode::default(), InterceptionMode::Extension);
    }

    #[test]
    fn test_mode_as_str() {
        assert_eq!(InterceptionMode::Extension.as_str(), "extension");
        assert_eq!(InterceptionMode::Proxy.as_str(), "proxy");
    }

    #[test]
    fn test_mode_name() {
        assert_eq!(InterceptionMode::Extension.name(), "Browser Extension");
        assert_eq!(InterceptionMode::Proxy.name(), "System Proxy");
    }

    #[test]
    fn test_mode_description() {
        assert!(InterceptionMode::Extension
            .description()
            .contains("browser"));
        assert!(InterceptionMode::Proxy.description().contains("all app"));
    }

    #[test]
    fn test_mode_setup_info() {
        assert!(InterceptionMode::Extension
            .setup_info()
            .contains("extension"));
        assert!(InterceptionMode::Proxy.setup_info().contains("CA"));
    }

    #[test]
    fn test_mode_coverage() {
        assert_eq!(InterceptionMode::Extension.coverage(), "Browser only");
        assert_eq!(InterceptionMode::Proxy.coverage(), "All applications");
    }

    #[test]
    fn test_mode_requires_ca_cert() {
        assert!(!InterceptionMode::Extension.requires_ca_cert());
        assert!(InterceptionMode::Proxy.requires_ca_cert());
    }

    #[test]
    fn test_mode_all() {
        let modes = InterceptionMode::all();
        assert_eq!(modes.len(), 2);
        assert!(modes.contains(&InterceptionMode::Extension));
        assert!(modes.contains(&InterceptionMode::Proxy));
    }

    #[test]
    fn test_mode_display() {
        assert_eq!(format!("{}", InterceptionMode::Extension), "extension");
        assert_eq!(format!("{}", InterceptionMode::Proxy), "proxy");
    }

    #[test]
    fn test_mode_serialization() {
        let mode = InterceptionMode::Proxy;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"proxy\"");

        let deserialized: InterceptionMode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, mode);
    }

    // ==================== InterceptionManager Tests ====================

    #[test]
    fn test_manager_default_mode() {
        let manager = InterceptionManager::new();
        assert_eq!(manager.mode(), InterceptionMode::Extension);
        assert!(manager.is_extension_mode());
        assert!(!manager.is_proxy_mode());
    }

    #[test]
    fn test_manager_with_mode() {
        let manager = InterceptionManager::with_mode(InterceptionMode::Proxy);
        assert_eq!(manager.mode(), InterceptionMode::Proxy);
        assert!(!manager.is_extension_mode());
        assert!(manager.is_proxy_mode());
    }

    #[test]
    fn test_set_mode_requires_auth() {
        let manager = InterceptionManager::new();
        let auth = AuthManager::new();
        let fake_token = SessionToken::new(); // Not registered with auth

        let result = manager.set_mode(InterceptionMode::Proxy, &fake_token, &auth);
        assert!(matches!(result, Err(InterceptionError::SessionInvalid)));
    }

    #[test]
    fn test_set_mode_with_valid_session() {
        let manager = InterceptionManager::new();
        let auth = AuthManager::new();
        let session = auth.create_session();

        let event = manager
            .set_mode(InterceptionMode::Proxy, &session, &auth)
            .unwrap();

        assert_eq!(manager.mode(), InterceptionMode::Proxy);
        assert!(matches!(
            event,
            InterceptionEvent::ModeChanged {
                from: InterceptionMode::Extension,
                to: InterceptionMode::Proxy
            }
        ));
    }

    #[test]
    fn test_set_mode_already_in_mode() {
        let manager = InterceptionManager::new();
        let auth = AuthManager::new();
        let session = auth.create_session();

        let result = manager.set_mode(InterceptionMode::Extension, &session, &auth);
        assert!(matches!(result, Err(InterceptionError::AlreadyInMode(_))));
    }

    #[test]
    fn test_restore_mode() {
        let manager = InterceptionManager::new();
        assert_eq!(manager.mode(), InterceptionMode::Extension);

        manager.restore_mode(InterceptionMode::Proxy);
        assert_eq!(manager.mode(), InterceptionMode::Proxy);
    }

    #[test]
    fn test_mode_info() {
        let manager = InterceptionManager::new();
        let info = manager.mode_info();

        assert_eq!(info.mode, InterceptionMode::Extension);
        assert_eq!(info.name, "Browser Extension");
        assert!(!info.requires_ca_cert);
    }

    #[test]
    fn test_clone_shares_state() {
        let manager1 = InterceptionManager::new();
        let manager2 = manager1.clone();
        let auth = AuthManager::new();
        let session = auth.create_session();

        manager1
            .set_mode(InterceptionMode::Proxy, &session, &auth)
            .unwrap();

        // Both should see proxy mode
        assert_eq!(manager2.mode(), InterceptionMode::Proxy);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let manager = InterceptionManager::new();
        let auth = AuthManager::new();

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let m = manager.clone();
                let a = auth.clone();
                thread::spawn(move || {
                    let session = a.create_session();
                    let target_mode = if i % 2 == 0 {
                        InterceptionMode::Proxy
                    } else {
                        InterceptionMode::Extension
                    };
                    let _ = m.set_mode(target_mode, &session, &a);
                    m.mode()
                })
            })
            .collect();

        for handle in handles {
            let _ = handle.join().unwrap();
        }

        // State should be one of the valid modes
        let mode = manager.mode();
        assert!(matches!(
            mode,
            InterceptionMode::Extension | InterceptionMode::Proxy
        ));
    }

    // ==================== Event Tests ====================

    #[test]
    fn test_interception_event_equality() {
        let event1 = InterceptionEvent::ModeChanged {
            from: InterceptionMode::Extension,
            to: InterceptionMode::Proxy,
        };
        let event2 = InterceptionEvent::ModeChanged {
            from: InterceptionMode::Extension,
            to: InterceptionMode::Proxy,
        };
        let event3 = InterceptionEvent::ModeChanged {
            from: InterceptionMode::Proxy,
            to: InterceptionMode::Extension,
        };

        assert_eq!(event1, event2);
        assert_ne!(event1, event3);
    }
}

//! Protection toggle module (F018).
//!
//! Provides protection state management with authentication-guarded toggling.
//!
//! ## States
//!
//! - **Active**: Protection is enabled and filtering content (default)
//! - **Paused**: Protection temporarily disabled, auto-resumes after duration
//! - **Disabled**: Protection off until manually re-enabled
//!
//! ## Usage
//!
//! ```
//! use aegis_core::protection::{ProtectionManager, ProtectionState, PauseDuration};
//! use aegis_core::auth::AuthManager;
//!
//! let auth = AuthManager::new();
//! let mut manager = ProtectionManager::new();
//!
//! // Check current state
//! assert_eq!(manager.state(), ProtectionState::Active);
//!
//! // Pause for 15 minutes (requires valid session)
//! let session = auth.create_session();
//! manager.pause(PauseDuration::Minutes(15), &session, &auth).unwrap();
//!
//! // Check remaining time
//! if let Some(remaining) = manager.pause_remaining() {
//!     println!("Resuming in {} seconds", remaining.as_secs());
//! }
//!
//! // Resume immediately
//! manager.resume();
//! ```

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::auth::{AuthManager, SessionToken};

/// Protection state errors.
#[derive(Debug, Error)]
pub enum ProtectionError {
    /// Authentication required for this operation.
    #[error("authentication required to change protection state")]
    AuthRequired,

    /// Session is invalid or expired.
    #[error("session is invalid or expired")]
    SessionInvalid,

    /// Cannot perform operation in current state.
    #[error("cannot {0} when protection is {1}")]
    InvalidTransition(String, String),
}

/// Result type for protection operations.
pub type Result<T> = std::result::Result<T, ProtectionError>;

/// Protection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProtectionState {
    /// Protection is active and filtering content.
    #[default]
    Active,

    /// Protection is temporarily paused (will auto-resume).
    Paused,

    /// Protection is disabled until manually re-enabled.
    Disabled,
}

impl ProtectionState {
    /// Returns true if protection is currently active.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns true if filtering should be bypassed.
    pub fn is_bypassed(&self) -> bool {
        matches!(self, Self::Paused | Self::Disabled)
    }

    /// Returns the state as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Disabled => "disabled",
        }
    }

    /// Returns a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Active => "Protection is active",
            Self::Paused => "Protection is paused",
            Self::Disabled => "Protection is disabled",
        }
    }
}

impl std::fmt::Display for ProtectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Duration for which to pause protection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum PauseDuration {
    /// Pause for a specific number of minutes.
    Minutes(u32),

    /// Pause for a specific number of hours.
    Hours(u32),

    /// Pause indefinitely (until manually resumed or disabled).
    Indefinite,
}

impl PauseDuration {
    /// Common preset: 5 minutes.
    pub const FIVE_MINUTES: Self = Self::Minutes(5);

    /// Common preset: 15 minutes.
    pub const FIFTEEN_MINUTES: Self = Self::Minutes(15);

    /// Common preset: 30 minutes.
    pub const THIRTY_MINUTES: Self = Self::Minutes(30);

    /// Common preset: 1 hour.
    pub const ONE_HOUR: Self = Self::Hours(1);

    /// Converts to a Duration, or None if indefinite.
    pub fn to_duration(&self) -> Option<Duration> {
        match self {
            Self::Minutes(m) => Some(Duration::from_secs(*m as u64 * 60)),
            Self::Hours(h) => Some(Duration::from_secs(*h as u64 * 60 * 60)),
            Self::Indefinite => None,
        }
    }

    /// Returns a human-readable description.
    pub fn description(&self) -> String {
        match self {
            Self::Minutes(1) => "1 minute".to_string(),
            Self::Minutes(m) => format!("{} minutes", m),
            Self::Hours(1) => "1 hour".to_string(),
            Self::Hours(h) => format!("{} hours", h),
            Self::Indefinite => "Until resumed".to_string(),
        }
    }
}

/// Events emitted when protection state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectionEvent {
    /// Protection state changed.
    StateChanged {
        /// Previous state.
        from: ProtectionState,
        /// New state.
        to: ProtectionState,
    },

    /// Pause timer expired, protection resumed.
    PauseExpired,
}

/// Internal state data for the protection manager.
#[derive(Debug)]
struct ProtectionData {
    /// Current protection state.
    state: ProtectionState,

    /// When the current pause started (if paused).
    pause_start: Option<Instant>,

    /// Duration of the current pause (if paused and timed).
    pause_duration: Option<Duration>,
}

impl Default for ProtectionData {
    fn default() -> Self {
        Self {
            state: ProtectionState::Active,
            pause_start: None,
            pause_duration: None,
        }
    }
}

impl ProtectionData {
    /// Returns the remaining pause time, if any.
    fn pause_remaining(&self) -> Option<Duration> {
        match (self.state, self.pause_start, self.pause_duration) {
            (ProtectionState::Paused, Some(start), Some(duration)) => {
                let elapsed = start.elapsed();
                if elapsed >= duration {
                    None // Expired
                } else {
                    Some(duration - elapsed)
                }
            }
            (ProtectionState::Paused, Some(_), None) => {
                // Indefinite pause
                None
            }
            _ => None,
        }
    }

    /// Checks if a timed pause has expired.
    fn is_pause_expired(&self) -> bool {
        match (self.state, self.pause_start, self.pause_duration) {
            (ProtectionState::Paused, Some(start), Some(duration)) => start.elapsed() >= duration,
            _ => false,
        }
    }
}

/// Manages protection state with authentication-guarded operations.
///
/// Thread-safe and clonable for use across async contexts.
#[derive(Debug, Clone, Default)]
pub struct ProtectionManager {
    data: Arc<RwLock<ProtectionData>>,
}

impl ProtectionManager {
    /// Creates a new protection manager with active state.
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(ProtectionData::default())),
        }
    }

    /// Returns the current protection state.
    ///
    /// Also checks for and handles expired pauses.
    pub fn state(&self) -> ProtectionState {
        let mut data = self.data.write().unwrap();

        // Check for expired pause
        if data.is_pause_expired() {
            data.state = ProtectionState::Active;
            data.pause_start = None;
            data.pause_duration = None;
        }

        data.state
    }

    /// Returns true if protection is currently active.
    pub fn is_active(&self) -> bool {
        self.state().is_active()
    }

    /// Returns true if filtering should be bypassed.
    pub fn is_bypassed(&self) -> bool {
        self.state().is_bypassed()
    }

    /// Returns the remaining pause time, if any.
    ///
    /// Returns `None` if:
    /// - Not paused
    /// - Paused indefinitely
    /// - Pause has expired
    pub fn pause_remaining(&self) -> Option<Duration> {
        let data = self.data.read().unwrap();
        data.pause_remaining()
    }

    /// Pauses protection for the specified duration.
    ///
    /// Requires a valid authenticated session.
    pub fn pause(
        &self,
        duration: PauseDuration,
        session: &SessionToken,
        auth: &AuthManager,
    ) -> Result<ProtectionEvent> {
        // Validate session
        if !auth.validate_session(session) {
            return Err(ProtectionError::SessionInvalid);
        }

        let mut data = self.data.write().unwrap();
        let from = data.state;

        // Set paused state
        data.state = ProtectionState::Paused;
        data.pause_start = Some(Instant::now());
        data.pause_duration = duration.to_duration();

        Ok(ProtectionEvent::StateChanged {
            from,
            to: ProtectionState::Paused,
        })
    }

    /// Resumes protection immediately.
    ///
    /// Does not require authentication (resuming is always allowed).
    pub fn resume(&self) -> Option<ProtectionEvent> {
        let mut data = self.data.write().unwrap();

        if data.state == ProtectionState::Active {
            return None; // Already active
        }

        let from = data.state;
        data.state = ProtectionState::Active;
        data.pause_start = None;
        data.pause_duration = None;

        Some(ProtectionEvent::StateChanged {
            from,
            to: ProtectionState::Active,
        })
    }

    /// Disables protection completely.
    ///
    /// Requires a valid authenticated session.
    /// Protection remains off until manually re-enabled.
    pub fn disable(&self, session: &SessionToken, auth: &AuthManager) -> Result<ProtectionEvent> {
        // Validate session
        if !auth.validate_session(session) {
            return Err(ProtectionError::SessionInvalid);
        }

        let mut data = self.data.write().unwrap();
        let from = data.state;

        data.state = ProtectionState::Disabled;
        data.pause_start = None;
        data.pause_duration = None;

        Ok(ProtectionEvent::StateChanged {
            from,
            to: ProtectionState::Disabled,
        })
    }

    /// Enables protection (sets to Active state).
    ///
    /// Does not require authentication (enabling is always allowed).
    pub fn enable(&self) -> Option<ProtectionEvent> {
        let mut data = self.data.write().unwrap();

        if data.state == ProtectionState::Active {
            return None; // Already active
        }

        let from = data.state;
        data.state = ProtectionState::Active;
        data.pause_start = None;
        data.pause_duration = None;

        Some(ProtectionEvent::StateChanged {
            from,
            to: ProtectionState::Active,
        })
    }

    /// Checks for expired pause and auto-resumes if needed.
    ///
    /// Returns `Some(ProtectionEvent::PauseExpired)` if auto-resumed.
    pub fn check_expiry(&self) -> Option<ProtectionEvent> {
        let mut data = self.data.write().unwrap();

        if data.is_pause_expired() {
            data.state = ProtectionState::Active;
            data.pause_start = None;
            data.pause_duration = None;
            Some(ProtectionEvent::PauseExpired)
        } else {
            None
        }
    }

    /// Sets state directly (for loading from storage).
    ///
    /// Note: This does not validate authentication.
    /// Use only when restoring state from persistent storage.
    pub fn set_state(&self, state: ProtectionState) {
        let mut data = self.data.write().unwrap();
        data.state = state;

        // Clear pause data if not paused
        if state != ProtectionState::Paused {
            data.pause_start = None;
            data.pause_duration = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration as StdDuration;

    // ==================== ProtectionState Tests ====================

    #[test]
    fn test_default_state_is_active() {
        assert_eq!(ProtectionState::default(), ProtectionState::Active);
    }

    #[test]
    fn test_state_is_active() {
        assert!(ProtectionState::Active.is_active());
        assert!(!ProtectionState::Paused.is_active());
        assert!(!ProtectionState::Disabled.is_active());
    }

    #[test]
    fn test_state_is_bypassed() {
        assert!(!ProtectionState::Active.is_bypassed());
        assert!(ProtectionState::Paused.is_bypassed());
        assert!(ProtectionState::Disabled.is_bypassed());
    }

    #[test]
    fn test_state_as_str() {
        assert_eq!(ProtectionState::Active.as_str(), "active");
        assert_eq!(ProtectionState::Paused.as_str(), "paused");
        assert_eq!(ProtectionState::Disabled.as_str(), "disabled");
    }

    #[test]
    fn test_state_display() {
        assert_eq!(format!("{}", ProtectionState::Active), "active");
        assert_eq!(format!("{}", ProtectionState::Paused), "paused");
        assert_eq!(format!("{}", ProtectionState::Disabled), "disabled");
    }

    #[test]
    fn test_state_serialization() {
        let state = ProtectionState::Paused;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"paused\"");

        let deserialized: ProtectionState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, state);
    }

    // ==================== PauseDuration Tests ====================

    #[test]
    fn test_pause_duration_to_duration() {
        assert_eq!(
            PauseDuration::Minutes(5).to_duration(),
            Some(StdDuration::from_secs(300))
        );
        assert_eq!(
            PauseDuration::Minutes(15).to_duration(),
            Some(StdDuration::from_secs(900))
        );
        assert_eq!(
            PauseDuration::Hours(1).to_duration(),
            Some(StdDuration::from_secs(3600))
        );
        assert_eq!(PauseDuration::Indefinite.to_duration(), None);
    }

    #[test]
    fn test_pause_duration_presets() {
        assert_eq!(
            PauseDuration::FIVE_MINUTES.to_duration(),
            Some(StdDuration::from_secs(300))
        );
        assert_eq!(
            PauseDuration::FIFTEEN_MINUTES.to_duration(),
            Some(StdDuration::from_secs(900))
        );
        assert_eq!(
            PauseDuration::THIRTY_MINUTES.to_duration(),
            Some(StdDuration::from_secs(1800))
        );
        assert_eq!(
            PauseDuration::ONE_HOUR.to_duration(),
            Some(StdDuration::from_secs(3600))
        );
    }

    #[test]
    fn test_pause_duration_description() {
        assert_eq!(PauseDuration::Minutes(1).description(), "1 minute");
        assert_eq!(PauseDuration::Minutes(5).description(), "5 minutes");
        assert_eq!(PauseDuration::Hours(1).description(), "1 hour");
        assert_eq!(PauseDuration::Hours(2).description(), "2 hours");
        assert_eq!(PauseDuration::Indefinite.description(), "Until resumed");
    }

    #[test]
    fn test_pause_duration_serialization() {
        let duration = PauseDuration::Minutes(15);
        let json = serde_json::to_string(&duration).unwrap();
        let deserialized: PauseDuration = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, duration);

        let indefinite = PauseDuration::Indefinite;
        let json = serde_json::to_string(&indefinite).unwrap();
        let deserialized: PauseDuration = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, indefinite);
    }

    // ==================== ProtectionManager Tests ====================

    #[test]
    fn test_manager_default_state() {
        let manager = ProtectionManager::new();
        assert_eq!(manager.state(), ProtectionState::Active);
        assert!(manager.is_active());
        assert!(!manager.is_bypassed());
    }

    #[test]
    fn test_pause_requires_auth() {
        let manager = ProtectionManager::new();
        let auth = AuthManager::new();
        let fake_token = SessionToken::new(); // Not registered with auth

        let result = manager.pause(PauseDuration::FIVE_MINUTES, &fake_token, &auth);
        assert!(matches!(result, Err(ProtectionError::SessionInvalid)));
    }

    #[test]
    fn test_pause_with_valid_session() {
        let manager = ProtectionManager::new();
        let auth = AuthManager::new();
        let session = auth.create_session();

        let event = manager
            .pause(PauseDuration::FIVE_MINUTES, &session, &auth)
            .unwrap();

        assert_eq!(manager.state(), ProtectionState::Paused);
        assert!(manager.is_bypassed());
        assert!(matches!(
            event,
            ProtectionEvent::StateChanged {
                from: ProtectionState::Active,
                to: ProtectionState::Paused
            }
        ));
    }

    #[test]
    fn test_pause_remaining() {
        let manager = ProtectionManager::new();
        let auth = AuthManager::new();
        let session = auth.create_session();

        manager
            .pause(PauseDuration::Minutes(1), &session, &auth)
            .unwrap();

        let remaining = manager.pause_remaining();
        assert!(remaining.is_some());

        // Should be roughly 60 seconds (allow some time for test execution)
        let secs = remaining.unwrap().as_secs();
        assert!(secs > 55 && secs <= 60);
    }

    #[test]
    fn test_indefinite_pause_no_remaining() {
        let manager = ProtectionManager::new();
        let auth = AuthManager::new();
        let session = auth.create_session();

        manager
            .pause(PauseDuration::Indefinite, &session, &auth)
            .unwrap();

        assert_eq!(manager.state(), ProtectionState::Paused);
        assert!(manager.pause_remaining().is_none());
    }

    #[test]
    fn test_resume() {
        let manager = ProtectionManager::new();
        let auth = AuthManager::new();
        let session = auth.create_session();

        manager
            .pause(PauseDuration::FIVE_MINUTES, &session, &auth)
            .unwrap();
        assert_eq!(manager.state(), ProtectionState::Paused);

        let event = manager.resume();
        assert!(event.is_some());
        assert_eq!(manager.state(), ProtectionState::Active);
        assert!(matches!(
            event.unwrap(),
            ProtectionEvent::StateChanged {
                from: ProtectionState::Paused,
                to: ProtectionState::Active
            }
        ));
    }

    #[test]
    fn test_resume_when_already_active() {
        let manager = ProtectionManager::new();

        let event = manager.resume();
        assert!(event.is_none()); // No change
        assert_eq!(manager.state(), ProtectionState::Active);
    }

    #[test]
    fn test_disable_requires_auth() {
        let manager = ProtectionManager::new();
        let auth = AuthManager::new();
        let fake_token = SessionToken::new();

        let result = manager.disable(&fake_token, &auth);
        assert!(matches!(result, Err(ProtectionError::SessionInvalid)));
    }

    #[test]
    fn test_disable_with_valid_session() {
        let manager = ProtectionManager::new();
        let auth = AuthManager::new();
        let session = auth.create_session();

        let event = manager.disable(&session, &auth).unwrap();

        assert_eq!(manager.state(), ProtectionState::Disabled);
        assert!(manager.is_bypassed());
        assert!(matches!(
            event,
            ProtectionEvent::StateChanged {
                from: ProtectionState::Active,
                to: ProtectionState::Disabled
            }
        ));
    }

    #[test]
    fn test_enable() {
        let manager = ProtectionManager::new();
        let auth = AuthManager::new();
        let session = auth.create_session();

        manager.disable(&session, &auth).unwrap();
        assert_eq!(manager.state(), ProtectionState::Disabled);

        let event = manager.enable();
        assert!(event.is_some());
        assert_eq!(manager.state(), ProtectionState::Active);
    }

    #[test]
    fn test_enable_when_already_active() {
        let manager = ProtectionManager::new();

        let event = manager.enable();
        assert!(event.is_none()); // No change
    }

    #[test]
    fn test_set_state() {
        let manager = ProtectionManager::new();

        manager.set_state(ProtectionState::Paused);
        assert_eq!(manager.state(), ProtectionState::Paused);

        manager.set_state(ProtectionState::Disabled);
        assert_eq!(manager.state(), ProtectionState::Disabled);

        manager.set_state(ProtectionState::Active);
        assert_eq!(manager.state(), ProtectionState::Active);
    }

    #[test]
    fn test_clone_shares_state() {
        let manager1 = ProtectionManager::new();
        let manager2 = manager1.clone();
        let auth = AuthManager::new();
        let session = auth.create_session();

        manager1
            .pause(PauseDuration::FIVE_MINUTES, &session, &auth)
            .unwrap();

        // Both should see paused state
        assert_eq!(manager2.state(), ProtectionState::Paused);
    }

    #[test]
    fn test_thread_safety() {
        let manager = ProtectionManager::new();
        let auth = AuthManager::new();

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let m = manager.clone();
                let a = auth.clone();
                thread::spawn(move || {
                    let session = a.create_session();
                    if i % 2 == 0 {
                        let _ = m.pause(PauseDuration::FIVE_MINUTES, &session, &a);
                    } else {
                        m.resume();
                    }
                    m.state()
                })
            })
            .collect();

        for handle in handles {
            let _ = handle.join().unwrap();
        }

        // State should be one of the valid states
        let state = manager.state();
        assert!(matches!(
            state,
            ProtectionState::Active | ProtectionState::Paused
        ));
    }

    // ==================== Auto-Resume Tests ====================

    #[test]
    fn test_pause_expiry_detection() {
        let manager = ProtectionManager::new();
        let auth = AuthManager::new();
        let session = auth.create_session();

        // Pause for a very short duration (1 second) for testing
        // Note: We can't use 0 seconds, so we'll manipulate the internal state
        manager
            .pause(PauseDuration::Minutes(1), &session, &auth)
            .unwrap();

        // Manually set a past start time to simulate expiry
        {
            let mut data = manager.data.write().unwrap();
            data.pause_start = Some(Instant::now() - StdDuration::from_secs(120));
            // 2 minutes ago
        }

        // check_expiry should detect and auto-resume
        let event = manager.check_expiry();
        assert!(matches!(event, Some(ProtectionEvent::PauseExpired)));
        assert_eq!(manager.state(), ProtectionState::Active);
    }

    #[test]
    fn test_state_auto_resumes_on_access() {
        let manager = ProtectionManager::new();
        let auth = AuthManager::new();
        let session = auth.create_session();

        manager
            .pause(PauseDuration::Minutes(1), &session, &auth)
            .unwrap();

        // Manually set a past start time to simulate expiry
        {
            let mut data = manager.data.write().unwrap();
            data.pause_start = Some(Instant::now() - StdDuration::from_secs(120));
        }

        // Accessing state() should auto-resume
        let state = manager.state();
        assert_eq!(state, ProtectionState::Active);
    }

    // ==================== Event Tests ====================

    #[test]
    fn test_protection_event_equality() {
        let event1 = ProtectionEvent::StateChanged {
            from: ProtectionState::Active,
            to: ProtectionState::Paused,
        };
        let event2 = ProtectionEvent::StateChanged {
            from: ProtectionState::Active,
            to: ProtectionState::Paused,
        };
        let event3 = ProtectionEvent::PauseExpired;

        assert_eq!(event1, event2);
        assert_ne!(event1, event3);
    }
}

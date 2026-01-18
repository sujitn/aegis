//! Centralized state manager for cross-process state synchronization (F032).
//!
//! This module provides a high-level interface for managing application state
//! that is shared across processes (main app, dashboard subprocess, proxy).
//! All state is persisted to the database for cross-process visibility.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::repository::state::StateChange;
use crate::{Database, ProtectionState, StorageError};

/// Error type for state manager operations.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("invalid state: {0}")]
    InvalidState(String),

    #[error("session expired")]
    SessionExpired,

    #[error("not authenticated")]
    NotAuthenticated,
}

/// Result type for state manager operations.
pub type Result<T> = std::result::Result<T, StateError>;

/// Centralized state manager backed by database.
///
/// This manager provides:
/// - Protection state (active/paused/disabled)
/// - Session management (cross-process authentication)
/// - State change notifications (cache invalidation)
///
/// All state is persisted to SQLite, making it visible to all processes.
#[derive(Clone)]
pub struct StateManager {
    db: Arc<Database>,
    /// Last known sequence number for change detection.
    last_seq: Arc<AtomicI64>,
    /// Identifier for this instance (for audit trail).
    instance_id: String,
}

impl StateManager {
    /// Creates a new state manager with the given database.
    pub fn new(db: Arc<Database>, instance_id: impl Into<String>) -> Self {
        let last_seq = db.get_latest_state_seq().unwrap_or(0);
        Self {
            db,
            last_seq: Arc::new(AtomicI64::new(last_seq)),
            instance_id: instance_id.into(),
        }
    }

    /// Returns the instance identifier.
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    // ==================== Protection State ====================

    /// Get current protection state.
    pub fn get_protection_state(&self) -> Result<ProtectionState> {
        Ok(self.db.get_protection_state()?)
    }

    /// Check if filtering is currently enabled.
    /// Takes pause expiry into account.
    pub fn is_filtering_enabled(&self) -> Result<bool> {
        Ok(self.db.is_filtering_enabled()?)
    }

    /// Pause protection for a specified duration.
    pub fn pause_protection(&self, duration: PauseDuration) -> Result<()> {
        let until = match duration {
            PauseDuration::Minutes(m) => Some(Utc::now() + Duration::minutes(m as i64)),
            PauseDuration::Hours(h) => Some(Utc::now() + Duration::hours(h as i64)),
            PauseDuration::Indefinite => None,
        };
        self.db.pause_protection(until, &self.instance_id)?;
        Ok(())
    }

    /// Resume protection immediately.
    pub fn resume_protection(&self) -> Result<()> {
        self.db.resume_protection(&self.instance_id)?;
        Ok(())
    }

    /// Disable protection completely (until explicitly re-enabled).
    pub fn disable_protection(&self) -> Result<()> {
        self.db.disable_protection(&self.instance_id)?;
        Ok(())
    }

    // ==================== Session Management ====================

    /// Create a new session token.
    /// Returns the token string.
    pub fn create_session(&self) -> Result<String> {
        let token = generate_session_token();
        let expires_in = Duration::minutes(15);
        self.db.create_session(&token, expires_in)?;
        Ok(token)
    }

    /// Create a session with custom expiry duration.
    pub fn create_session_with_expiry(&self, expires_in: Duration) -> Result<String> {
        let token = generate_session_token();
        self.db.create_session(&token, expires_in)?;
        Ok(token)
    }

    /// Validate a session token.
    /// Returns true if valid, false if expired or invalid.
    pub fn validate_session(&self, token: &str) -> Result<bool> {
        Ok(self.db.validate_session(token)?)
    }

    /// Invalidate a session token (logout).
    pub fn invalidate_session(&self, token: &str) -> Result<()> {
        self.db.invalidate_session(token)?;
        Ok(())
    }

    /// Clean up expired sessions.
    /// Returns number of sessions cleaned up.
    pub fn cleanup_sessions(&self) -> Result<u64> {
        Ok(self.db.cleanup_expired_sessions()?)
    }

    // ==================== State Change Notifications ====================

    /// Check for state changes since last poll.
    /// Returns true if there are changes.
    pub fn has_changes(&self) -> Result<bool> {
        let last = self.last_seq.load(Ordering::Relaxed);
        if let Some(new_seq) = self.db.poll_state_changes(last)? {
            self.last_seq.store(new_seq, Ordering::Relaxed);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get the current sequence number.
    pub fn current_seq(&self) -> i64 {
        self.last_seq.load(Ordering::Relaxed)
    }

    /// Update the tracked sequence number.
    pub fn set_seq(&self, seq: i64) {
        self.last_seq.store(seq, Ordering::Relaxed);
    }

    /// Get all changes since a given sequence number.
    pub fn get_changes_since(&self, since_seq: i64) -> Result<Vec<StateChange>> {
        Ok(self.db.get_state_changes_since(since_seq)?)
    }

    /// Record a custom state change (for cache invalidation).
    pub fn record_change(&self, key: &str) -> Result<i64> {
        Ok(self.db.record_state_change(key)?)
    }

    // ==================== Generic App State ====================

    /// Get a generic app state value.
    pub fn get_state(&self, key: &str) -> Result<Option<String>> {
        Ok(self.db.get_app_state(key)?)
    }

    /// Set a generic app state value.
    pub fn set_state(&self, key: &str, value: &str) -> Result<()> {
        self.db.set_app_state(key, value, &self.instance_id)?;
        Ok(())
    }
}

/// Pause duration options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseDuration {
    /// Pause for a number of minutes.
    Minutes(u32),
    /// Pause for a number of hours.
    Hours(u32),
    /// Pause indefinitely (until manually resumed).
    Indefinite,
}

impl PauseDuration {
    /// 5 minutes pause.
    pub const FIVE_MINUTES: Self = Self::Minutes(5);
    /// 15 minutes pause.
    pub const FIFTEEN_MINUTES: Self = Self::Minutes(15);
    /// 30 minutes pause.
    pub const THIRTY_MINUTES: Self = Self::Minutes(30);
    /// 1 hour pause.
    pub const ONE_HOUR: Self = Self::Hours(1);
}

/// Generate a random session token.
fn generate_session_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    // Simple token: timestamp + random suffix
    // In production, use a cryptographically secure random generator
    let random: u64 = (timestamp as u64).wrapping_mul(0x5DEECE66D).wrapping_add(0xB);
    format!("sess_{}_{:016x}", timestamp, random)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> StateManager {
        let db = Database::in_memory().unwrap();
        StateManager::new(Arc::new(db), "test")
    }

    #[test]
    fn test_protection_state() {
        let mgr = create_test_manager();

        // Default is active
        assert!(mgr.is_filtering_enabled().unwrap());

        // Pause
        mgr.pause_protection(PauseDuration::FIFTEEN_MINUTES).unwrap();
        assert!(!mgr.is_filtering_enabled().unwrap());

        // Resume
        mgr.resume_protection().unwrap();
        assert!(mgr.is_filtering_enabled().unwrap());

        // Disable
        mgr.disable_protection().unwrap();
        assert!(!mgr.is_filtering_enabled().unwrap());

        // Resume from disabled
        mgr.resume_protection().unwrap();
        assert!(mgr.is_filtering_enabled().unwrap());
    }

    #[test]
    fn test_session_management() {
        let mgr = create_test_manager();

        // Create session
        let token = mgr.create_session().unwrap();
        assert!(token.starts_with("sess_"));

        // Validate session
        assert!(mgr.validate_session(&token).unwrap());

        // Invalidate session
        mgr.invalidate_session(&token).unwrap();
        assert!(!mgr.validate_session(&token).unwrap());
    }

    #[test]
    fn test_state_changes() {
        let mgr = create_test_manager();

        // Record initial seq
        let initial_seq = mgr.current_seq();

        // Record a change
        mgr.record_change("test_key").unwrap();

        // Should detect change
        assert!(mgr.has_changes().unwrap());

        // Should not detect another change (already updated seq)
        assert!(!mgr.has_changes().unwrap());

        // Get changes since initial
        let changes = mgr.get_changes_since(initial_seq).unwrap();
        assert!(!changes.is_empty());
    }

    #[test]
    fn test_generic_state() {
        let mgr = create_test_manager();

        // Set state
        mgr.set_state("custom_key", "custom_value").unwrap();

        // Get state
        let value = mgr.get_state("custom_key").unwrap();
        assert_eq!(value, Some("custom_value".to_string()));

        // Non-existent key
        let none = mgr.get_state("nonexistent").unwrap();
        assert!(none.is_none());
    }
}

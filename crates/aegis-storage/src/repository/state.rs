//! Centralized state repository for cross-process state management (F032).
//!
//! This module provides database-backed state storage for:
//! - Protection status (active/paused/disabled)
//! - Session tokens (cross-process authentication)
//! - State change notifications (cache invalidation)

use chrono::{DateTime, Duration, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::database::Database;
use crate::error::Result;

/// Protection status stored in database.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProtectionState {
    /// Current status: "active", "paused", or "disabled"
    pub status: String,
    /// When pause expires (ISO 8601), null for indefinite or active
    pub pause_until: Option<String>,
}

impl Default for ProtectionState {
    fn default() -> Self {
        Self {
            status: "active".to_string(),
            pause_until: None,
        }
    }
}

impl ProtectionState {
    /// Creates an active protection state.
    pub fn active() -> Self {
        Self {
            status: "active".to_string(),
            pause_until: None,
        }
    }

    /// Creates a paused protection state with optional expiry.
    pub fn paused(until: Option<DateTime<Utc>>) -> Self {
        Self {
            status: "paused".to_string(),
            pause_until: until.map(|t| t.to_rfc3339()),
        }
    }

    /// Creates a disabled protection state.
    pub fn disabled() -> Self {
        Self {
            status: "disabled".to_string(),
            pause_until: None,
        }
    }

    /// Returns true if protection is currently active.
    pub fn is_active(&self) -> bool {
        if self.status == "active" {
            return true;
        }
        // Check if pause has expired
        if self.status == "paused" {
            if let Some(ref until) = self.pause_until {
                if let Ok(expiry) = DateTime::parse_from_rfc3339(until) {
                    if Utc::now() > expiry {
                        return true; // Pause expired
                    }
                }
            }
        }
        false
    }

    /// Returns true if protection is paused (and not expired).
    pub fn is_paused(&self) -> bool {
        if self.status != "paused" {
            return false;
        }
        // Check if pause has expired
        if let Some(ref until) = self.pause_until {
            if let Ok(expiry) = DateTime::parse_from_rfc3339(until) {
                return Utc::now() <= expiry;
            }
        }
        // Indefinite pause
        true
    }

    /// Returns true if protection is disabled.
    pub fn is_disabled(&self) -> bool {
        self.status == "disabled"
    }
}

/// Session record stored in database.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
}

/// State change record for cache invalidation.
#[derive(Debug, Clone)]
pub struct StateChange {
    pub seq: i64,
    pub state_key: String,
    pub changed_at: DateTime<Utc>,
}

impl Database {
    // ==================== Protection State ====================

    /// Get current protection state from database.
    pub fn get_protection_state(&self) -> Result<ProtectionState> {
        let conn = self.pool.get()?;
        let value: Option<String> = conn
            .query_row(
                "SELECT value FROM app_state WHERE key = 'protection'",
                [],
                |row| row.get(0),
            )
            .ok();

        match value {
            Some(json) => {
                let state: ProtectionState = serde_json::from_str(&json).unwrap_or_default();
                Ok(state)
            }
            None => Ok(ProtectionState::default()),
        }
    }

    /// Set protection state in database.
    pub fn set_protection_state(&self, state: &ProtectionState, updated_by: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let json = serde_json::to_string(state)?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT OR REPLACE INTO app_state (key, value, updated_at, updated_by) VALUES ('protection', ?1, ?2, ?3)",
            params![json, now, updated_by],
        )?;

        // Record state change for cache invalidation
        conn.execute(
            "INSERT INTO state_changes (state_key, changed_at) VALUES ('protection', ?1)",
            params![now],
        )?;

        Ok(())
    }

    /// Pause protection until specified time (or indefinitely).
    pub fn pause_protection(&self, until: Option<DateTime<Utc>>, updated_by: &str) -> Result<()> {
        let state = ProtectionState::paused(until);
        self.set_protection_state(&state, updated_by)
    }

    /// Resume protection (set to active).
    pub fn resume_protection(&self, updated_by: &str) -> Result<()> {
        let state = ProtectionState::active();
        self.set_protection_state(&state, updated_by)
    }

    /// Disable protection completely.
    pub fn disable_protection(&self, updated_by: &str) -> Result<()> {
        let state = ProtectionState::disabled();
        self.set_protection_state(&state, updated_by)
    }

    /// Check if filtering should be enabled (considering pause expiry).
    pub fn is_filtering_enabled(&self) -> Result<bool> {
        let state = self.get_protection_state()?;
        Ok(state.is_active())
    }

    // ==================== Session Management ====================

    /// Create a new session token with expiry.
    pub fn create_session(&self, token: &str, expires_in: Duration) -> Result<()> {
        let conn = self.pool.get()?;
        let now = Utc::now();
        let expires_at = now + expires_in;

        conn.execute(
            "INSERT OR REPLACE INTO sessions (token, created_at, expires_at, last_used_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                token,
                now.to_rfc3339(),
                expires_at.to_rfc3339(),
                now.to_rfc3339()
            ],
        )?;

        Ok(())
    }

    /// Validate session token (checks expiry, updates last_used).
    /// Returns true if session is valid.
    pub fn validate_session(&self, token: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        // Get session and check expiry
        let result: Option<String> = conn
            .query_row(
                "SELECT expires_at FROM sessions WHERE token = ?1",
                params![token],
                |row| row.get(0),
            )
            .ok();

        match result {
            Some(expires_at_str) => {
                if let Ok(expires_at) = DateTime::parse_from_rfc3339(&expires_at_str) {
                    if now > expires_at {
                        // Session expired, delete it
                        conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])?;
                        return Ok(false);
                    }

                    // Update last_used_at
                    conn.execute(
                        "UPDATE sessions SET last_used_at = ?1 WHERE token = ?2",
                        params![now_str, token],
                    )?;

                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            None => Ok(false),
        }
    }

    /// Invalidate (delete) a session token.
    pub fn invalidate_session(&self, token: &str) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])?;
        Ok(())
    }

    /// Clean up all expired sessions.
    /// Returns the number of sessions deleted.
    pub fn cleanup_expired_sessions(&self) -> Result<u64> {
        let conn = self.pool.get()?;
        let now = Utc::now().to_rfc3339();
        let deleted = conn.execute("DELETE FROM sessions WHERE expires_at < ?1", params![now])?;
        Ok(deleted as u64)
    }

    /// Get all active (non-expired) sessions.
    pub fn get_active_sessions(&self) -> Result<Vec<SessionRecord>> {
        let conn = self.pool.get()?;
        let now = Utc::now().to_rfc3339();

        let mut stmt = conn.prepare(
            "SELECT token, created_at, expires_at, last_used_at FROM sessions WHERE expires_at > ?1",
        )?;

        let sessions = stmt
            .query_map(params![now], |row| {
                let token: String = row.get(0)?;
                let created_at: String = row.get(1)?;
                let expires_at: String = row.get(2)?;
                let last_used_at: String = row.get(3)?;

                Ok(SessionRecord {
                    token,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    expires_at: DateTime::parse_from_rfc3339(&expires_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    last_used_at: DateTime::parse_from_rfc3339(&last_used_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    // ==================== State Change Notifications ====================

    /// Get the latest state change sequence number.
    pub fn get_latest_state_seq(&self) -> Result<i64> {
        let conn = self.pool.get()?;
        let seq: Option<i64> = conn
            .query_row("SELECT MAX(seq) FROM state_changes", [], |row| row.get(0))
            .ok()
            .flatten();
        Ok(seq.unwrap_or(0))
    }

    /// Get state changes since a given sequence number.
    pub fn get_state_changes_since(&self, since_seq: i64) -> Result<Vec<StateChange>> {
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare(
            "SELECT seq, state_key, changed_at FROM state_changes WHERE seq > ?1 ORDER BY seq",
        )?;

        let changes = stmt
            .query_map(params![since_seq], |row| {
                let seq: i64 = row.get(0)?;
                let state_key: String = row.get(1)?;
                let changed_at: String = row.get(2)?;

                Ok(StateChange {
                    seq,
                    state_key,
                    changed_at: DateTime::parse_from_rfc3339(&changed_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(changes)
    }

    /// Check if there are state changes since last check.
    /// Returns Some(new_seq) if changes detected, None otherwise.
    pub fn poll_state_changes(&self, last_seq: i64) -> Result<Option<i64>> {
        let current_seq = self.get_latest_state_seq()?;
        if current_seq > last_seq {
            Ok(Some(current_seq))
        } else {
            Ok(None)
        }
    }

    /// Record a state change for cache invalidation.
    pub fn record_state_change(&self, state_key: &str) -> Result<i64> {
        let conn = self.pool.get()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO state_changes (state_key, changed_at) VALUES (?1, ?2)",
            params![state_key, now],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Clean up old state changes (keep only recent ones).
    /// Returns the number of records deleted.
    pub fn cleanup_old_state_changes(&self, keep_count: i64) -> Result<u64> {
        let conn = self.pool.get()?;

        // Get the seq threshold
        let threshold: Option<i64> = conn
            .query_row(
                "SELECT seq FROM state_changes ORDER BY seq DESC LIMIT 1 OFFSET ?1",
                params![keep_count],
                |row| row.get(0),
            )
            .ok();

        if let Some(threshold_seq) = threshold {
            let deleted = conn.execute(
                "DELETE FROM state_changes WHERE seq <= ?1",
                params![threshold_seq],
            )?;
            Ok(deleted as u64)
        } else {
            Ok(0)
        }
    }

    // ==================== Generic App State ====================

    /// Get a generic app state value by key.
    pub fn get_app_state(&self, key: &str) -> Result<Option<String>> {
        let conn = self.pool.get()?;
        let value: Option<String> = conn
            .query_row(
                "SELECT value FROM app_state WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .ok();
        Ok(value)
    }

    /// Set a generic app state value.
    pub fn set_app_state(&self, key: &str, value: &str, updated_by: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT OR REPLACE INTO app_state (key, value, updated_at, updated_by) VALUES (?1, ?2, ?3, ?4)",
            params![key, value, now, updated_by],
        )?;

        // Record state change
        conn.execute(
            "INSERT INTO state_changes (state_key, changed_at) VALUES (?1, ?2)",
            params![key, now],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protection_state_default() {
        let state = ProtectionState::default();
        assert!(state.is_active());
        assert!(!state.is_paused());
        assert!(!state.is_disabled());
    }

    #[test]
    fn test_protection_state_paused() {
        let future = Utc::now() + Duration::hours(1);
        let state = ProtectionState::paused(Some(future));
        assert!(!state.is_active());
        assert!(state.is_paused());
        assert!(!state.is_disabled());
    }

    #[test]
    fn test_protection_state_paused_expired() {
        let past = Utc::now() - Duration::hours(1);
        let state = ProtectionState::paused(Some(past));
        // Expired pause should be treated as active
        assert!(state.is_active());
        assert!(!state.is_paused());
    }

    #[test]
    fn test_protection_state_disabled() {
        let state = ProtectionState::disabled();
        assert!(!state.is_active());
        assert!(!state.is_paused());
        assert!(state.is_disabled());
    }

    #[test]
    fn test_db_protection_state() {
        let db = Database::in_memory().unwrap();

        // Default should be active
        let state = db.get_protection_state().unwrap();
        assert!(state.is_active());

        // Pause protection
        let future = Utc::now() + Duration::hours(1);
        db.pause_protection(Some(future), "test").unwrap();
        let state = db.get_protection_state().unwrap();
        assert!(state.is_paused());

        // Resume protection
        db.resume_protection("test").unwrap();
        let state = db.get_protection_state().unwrap();
        assert!(state.is_active());

        // Disable protection
        db.disable_protection("test").unwrap();
        let state = db.get_protection_state().unwrap();
        assert!(state.is_disabled());
    }

    #[test]
    fn test_db_session_management() {
        let db = Database::in_memory().unwrap();

        let token = "test_session_token_12345";

        // Create session
        db.create_session(token, Duration::hours(1)).unwrap();

        // Validate session
        assert!(db.validate_session(token).unwrap());

        // Invalidate session
        db.invalidate_session(token).unwrap();
        assert!(!db.validate_session(token).unwrap());
    }

    #[test]
    fn test_db_session_expiry() {
        let db = Database::in_memory().unwrap();

        let token = "expired_token";

        // Create session that's already expired
        db.create_session(token, Duration::seconds(-1)).unwrap();

        // Should not validate
        assert!(!db.validate_session(token).unwrap());
    }

    #[test]
    fn test_db_state_changes() {
        let db = Database::in_memory().unwrap();

        // Initial seq should be low (just the protection init)
        let initial_seq = db.get_latest_state_seq().unwrap();

        // Record a change
        db.record_state_change("test_key").unwrap();

        // Seq should increase
        let new_seq = db.get_latest_state_seq().unwrap();
        assert!(new_seq > initial_seq);

        // Poll should detect change
        assert!(db.poll_state_changes(initial_seq).unwrap().is_some());
        assert!(db.poll_state_changes(new_seq).unwrap().is_none());
    }

    #[test]
    fn test_db_generic_app_state() {
        let db = Database::in_memory().unwrap();

        // Set value
        db.set_app_state("test_key", "test_value", "test").unwrap();

        // Get value
        let value = db.get_app_state("test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        // Non-existent key
        let none = db.get_app_state("nonexistent").unwrap();
        assert!(none.is_none());
    }
}

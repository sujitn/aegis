//! Parent authentication module (F013).
//!
//! Provides password-based authentication for protecting settings and rules.
//!
//! ## Features
//!
//! - Password validation (minimum 6 characters)
//! - Argon2 password hashing
//! - Session management with 15-minute timeout
//!
//! ## Usage
//!
//! ```
//! use aegis_core::auth::{AuthManager, AuthError};
//!
//! let auth = AuthManager::new();
//!
//! // Set password on first run
//! let hash = auth.hash_password("secret123").unwrap();
//!
//! // Verify password
//! assert!(auth.verify_password("secret123", &hash).unwrap());
//!
//! // Create a session after successful login
//! let session = auth.create_session();
//! assert!(auth.validate_session(&session));
//! ```

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Minimum password length requirement.
pub const MIN_PASSWORD_LENGTH: usize = 6;

/// Session timeout duration (15 minutes).
pub const SESSION_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// Authentication errors.
#[derive(Debug, Error)]
pub enum AuthError {
    /// Password is too short.
    #[error("password must be at least {MIN_PASSWORD_LENGTH} characters")]
    PasswordTooShort,

    /// Password is empty.
    #[error("password cannot be empty")]
    PasswordEmpty,

    /// Password hashing failed.
    #[error("failed to hash password: {0}")]
    HashingFailed(String),

    /// Password verification failed (invalid hash format).
    #[error("failed to verify password: {0}")]
    VerificationFailed(String),

    /// Session expired or invalid.
    #[error("session expired or invalid")]
    SessionInvalid,

    /// Authentication required (no valid session).
    #[error("authentication required")]
    AuthRequired,

    /// Password not set (first-run setup required).
    #[error("password not set - setup required")]
    NotSetup,
}

/// Result type for authentication operations.
pub type Result<T> = std::result::Result<T, AuthError>;

/// A session token representing an authenticated user.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionToken(String);

impl SessionToken {
    /// Create a new random session token.
    pub fn new() -> Self {
        let salt = SaltString::generate(&mut OsRng);
        Self(salt.to_string())
    }

    /// Create a session token from an existing string.
    ///
    /// Used for reconstructing tokens from API requests.
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the token as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SessionToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal session data with expiry tracking.
#[derive(Debug, Clone)]
struct SessionData {
    /// When the session was last used.
    last_used: Instant,
}

impl SessionData {
    fn new() -> Self {
        Self {
            last_used: Instant::now(),
        }
    }

    fn is_expired(&self) -> bool {
        self.last_used.elapsed() > SESSION_TIMEOUT
    }

    fn touch(&mut self) {
        self.last_used = Instant::now();
    }
}

/// Manages active sessions with automatic expiry.
#[derive(Debug, Default)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionToken, SessionData>>>,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new session and return its token.
    pub fn create_session(&self) -> SessionToken {
        let token = SessionToken::new();
        let data = SessionData::new();

        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(token.clone(), data);

        // Clean up expired sessions while we have the lock
        sessions.retain(|_, data| !data.is_expired());

        token
    }

    /// Validate a session token and refresh its expiry if valid.
    pub fn validate_session(&self, token: &SessionToken) -> bool {
        let mut sessions = self.sessions.write().unwrap();

        if let Some(data) = sessions.get_mut(token) {
            if data.is_expired() {
                sessions.remove(token);
                return false;
            }
            data.touch();
            return true;
        }

        false
    }

    /// Check if a session is valid without refreshing its expiry.
    pub fn is_session_valid(&self, token: &SessionToken) -> bool {
        let sessions = self.sessions.read().unwrap();

        if let Some(data) = sessions.get(token) {
            return !data.is_expired();
        }

        false
    }

    /// Invalidate (logout) a session.
    pub fn invalidate_session(&self, token: &SessionToken) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.remove(token);
    }

    /// Invalidate all sessions (logout everywhere).
    pub fn invalidate_all(&self) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.clear();
    }

    /// Get the number of active (non-expired) sessions.
    pub fn active_session_count(&self) -> usize {
        let sessions = self.sessions.read().unwrap();
        sessions.values().filter(|d| !d.is_expired()).count()
    }

    /// Clean up expired sessions.
    pub fn cleanup_expired(&self) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.retain(|_, data| !data.is_expired());
    }
}

impl Clone for SessionManager {
    fn clone(&self) -> Self {
        Self {
            sessions: Arc::clone(&self.sessions),
        }
    }
}

/// Main authentication manager.
///
/// Provides password hashing, verification, and session management.
#[derive(Debug, Clone, Default)]
pub struct AuthManager {
    sessions: SessionManager,
}

impl AuthManager {
    /// Create a new authentication manager.
    pub fn new() -> Self {
        Self {
            sessions: SessionManager::new(),
        }
    }

    /// Validate a password meets requirements.
    pub fn validate_password(password: &str) -> Result<()> {
        if password.is_empty() {
            return Err(AuthError::PasswordEmpty);
        }

        if password.len() < MIN_PASSWORD_LENGTH {
            return Err(AuthError::PasswordTooShort);
        }

        Ok(())
    }

    /// Hash a password using Argon2.
    ///
    /// Returns the hashed password as a PHC string format.
    pub fn hash_password(&self, password: &str) -> Result<String> {
        Self::validate_password(password)?;

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::HashingFailed(e.to_string()))?;

        Ok(hash.to_string())
    }

    /// Verify a password against a stored hash.
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| AuthError::VerificationFailed(e.to_string()))?;

        let argon2 = Argon2::default();

        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(AuthError::VerificationFailed(e.to_string())),
        }
    }

    /// Create a new authenticated session.
    ///
    /// Call this after successful password verification.
    pub fn create_session(&self) -> SessionToken {
        self.sessions.create_session()
    }

    /// Validate a session token and refresh its expiry.
    pub fn validate_session(&self, token: &SessionToken) -> bool {
        self.sessions.validate_session(token)
    }

    /// Check if a session is valid without refreshing its expiry.
    pub fn is_session_valid(&self, token: &SessionToken) -> bool {
        self.sessions.is_session_valid(token)
    }

    /// Logout a session.
    pub fn logout(&self, token: &SessionToken) {
        self.sessions.invalidate_session(token);
    }

    /// Logout all sessions.
    pub fn logout_all(&self) {
        self.sessions.invalidate_all();
    }

    /// Get the number of active sessions.
    pub fn active_session_count(&self) -> usize {
        self.sessions.active_session_count()
    }

    /// Get a reference to the session manager.
    pub fn session_manager(&self) -> &SessionManager {
        &self.sessions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    // ==================== Password Validation Tests ====================

    #[test]
    fn test_password_validation_success() {
        assert!(AuthManager::validate_password("123456").is_ok());
        assert!(AuthManager::validate_password("abcdef").is_ok());
        assert!(AuthManager::validate_password("password123!").is_ok());
        assert!(AuthManager::validate_password("a".repeat(100).as_str()).is_ok());
    }

    #[test]
    fn test_password_too_short() {
        let err = AuthManager::validate_password("12345").unwrap_err();
        assert!(matches!(err, AuthError::PasswordTooShort));
    }

    #[test]
    fn test_password_empty() {
        let err = AuthManager::validate_password("").unwrap_err();
        assert!(matches!(err, AuthError::PasswordEmpty));
    }

    #[test]
    fn test_password_exactly_min_length() {
        assert!(AuthManager::validate_password("123456").is_ok());
    }

    // ==================== Password Hashing Tests ====================

    #[test]
    fn test_hash_password_success() {
        let auth = AuthManager::new();
        let hash = auth.hash_password("password123").unwrap();

        // Hash should be in PHC format
        assert!(hash.starts_with("$argon2"));
        assert!(hash.contains("$"));
    }

    #[test]
    fn test_hash_password_unique_salts() {
        let auth = AuthManager::new();
        let hash1 = auth.hash_password("password123").unwrap();
        let hash2 = auth.hash_password("password123").unwrap();

        // Same password should produce different hashes (unique salts)
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_password_too_short() {
        let auth = AuthManager::new();
        let err = auth.hash_password("12345").unwrap_err();
        assert!(matches!(err, AuthError::PasswordTooShort));
    }

    // ==================== Password Verification Tests ====================

    #[test]
    fn test_verify_password_correct() {
        let auth = AuthManager::new();
        let hash = auth.hash_password("secret123").unwrap();

        assert!(auth.verify_password("secret123", &hash).unwrap());
    }

    #[test]
    fn test_verify_password_incorrect() {
        let auth = AuthManager::new();
        let hash = auth.hash_password("secret123").unwrap();

        assert!(!auth.verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let auth = AuthManager::new();
        let err = auth
            .verify_password("password", "not_a_valid_hash")
            .unwrap_err();
        assert!(matches!(err, AuthError::VerificationFailed(_)));
    }

    #[test]
    fn test_verify_password_empty_hash() {
        let auth = AuthManager::new();
        let err = auth.verify_password("password", "").unwrap_err();
        assert!(matches!(err, AuthError::VerificationFailed(_)));
    }

    // ==================== Session Tests ====================

    #[test]
    fn test_create_session() {
        let auth = AuthManager::new();
        let token = auth.create_session();

        assert!(!token.as_str().is_empty());
        assert!(auth.validate_session(&token));
    }

    #[test]
    fn test_session_tokens_unique() {
        let auth = AuthManager::new();
        let token1 = auth.create_session();
        let token2 = auth.create_session();

        assert_ne!(token1, token2);
    }

    #[test]
    fn test_validate_invalid_session() {
        let auth = AuthManager::new();
        let fake_token = SessionToken::new();

        assert!(!auth.validate_session(&fake_token));
    }

    #[test]
    fn test_logout_session() {
        let auth = AuthManager::new();
        let token = auth.create_session();

        assert!(auth.validate_session(&token));
        auth.logout(&token);
        assert!(!auth.validate_session(&token));
    }

    #[test]
    fn test_logout_all_sessions() {
        let auth = AuthManager::new();
        let token1 = auth.create_session();
        let token2 = auth.create_session();

        auth.logout_all();

        assert!(!auth.validate_session(&token1));
        assert!(!auth.validate_session(&token2));
    }

    #[test]
    fn test_active_session_count() {
        let auth = AuthManager::new();
        assert_eq!(auth.active_session_count(), 0);

        let _token1 = auth.create_session();
        assert_eq!(auth.active_session_count(), 1);

        let _token2 = auth.create_session();
        assert_eq!(auth.active_session_count(), 2);

        auth.logout_all();
        assert_eq!(auth.active_session_count(), 0);
    }

    #[test]
    fn test_is_session_valid_without_refresh() {
        let auth = AuthManager::new();
        let token = auth.create_session();

        assert!(auth.is_session_valid(&token));

        auth.logout(&token);
        assert!(!auth.is_session_valid(&token));
    }

    // ==================== Session Manager Tests ====================

    #[test]
    fn test_session_manager_cleanup() {
        let manager = SessionManager::new();
        let _token = manager.create_session();

        manager.cleanup_expired();
        // Session should still be valid (not expired)
        assert_eq!(manager.active_session_count(), 1);
    }

    #[test]
    fn test_session_manager_clone_shares_state() {
        let manager1 = SessionManager::new();
        let manager2 = manager1.clone();

        let token = manager1.create_session();

        // Both managers should see the same session
        assert!(manager2.validate_session(&token));
    }

    // ==================== Session Token Tests ====================

    #[test]
    fn test_session_token_serialization() {
        let token = SessionToken::new();
        let json = serde_json::to_string(&token).unwrap();
        let deserialized: SessionToken = serde_json::from_str(&json).unwrap();

        assert_eq!(token, deserialized);
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_full_auth_flow() {
        let auth = AuthManager::new();

        // 1. Set password on first run
        let password = "parent_password123";
        let hash = auth.hash_password(password).unwrap();

        // 2. Later: user tries to access settings
        // 3. Verify password
        assert!(auth.verify_password(password, &hash).unwrap());

        // 4. Create session after successful login
        let session = auth.create_session();

        // 5. Use session for subsequent requests
        assert!(auth.validate_session(&session));

        // 6. Eventually logout
        auth.logout(&session);
        assert!(!auth.validate_session(&session));
    }

    #[test]
    fn test_wrong_password_no_session() {
        let auth = AuthManager::new();

        let hash = auth.hash_password("correct_password").unwrap();

        // Wrong password should not allow creating a session
        let is_valid = auth.verify_password("wrong_password", &hash).unwrap();
        assert!(!is_valid);

        // Don't create session on failed login
        assert_eq!(auth.active_session_count(), 0);
    }

    #[test]
    fn test_multiple_concurrent_sessions() {
        let auth = AuthManager::new();

        // User can have multiple active sessions (multiple devices/browsers)
        let session1 = auth.create_session();
        let session2 = auth.create_session();
        let session3 = auth.create_session();

        assert!(auth.validate_session(&session1));
        assert!(auth.validate_session(&session2));
        assert!(auth.validate_session(&session3));

        assert_eq!(auth.active_session_count(), 3);
    }

    #[test]
    fn test_auth_manager_default() {
        let auth = AuthManager::default();
        // Should work the same as new()
        let hash = auth.hash_password("password123").unwrap();
        assert!(auth.verify_password("password123", &hash).unwrap());
    }

    // ==================== Thread Safety Tests ====================

    #[test]
    fn test_session_manager_thread_safe() {
        let auth = AuthManager::new();

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let auth_clone = auth.clone();
                thread::spawn(move || {
                    let token = auth_clone.create_session();
                    auth_clone.validate_session(&token)
                })
            })
            .collect();

        for handle in handles {
            assert!(handle.join().unwrap());
        }

        assert_eq!(auth.active_session_count(), 10);
    }
}

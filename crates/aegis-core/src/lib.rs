//! Aegis Core - Classification, rules, and authentication logic.
//!
//! This crate provides the core functionality for the Aegis AI safety platform.
//!
//! ## Modules
//!
//! - [`auth`] - Parent authentication with password hashing and sessions (F013)
//! - [`classifier`] - Content classification (keywords, ML, tiered pipeline)
//! - [`time_rules`] - Time-based blocking rules (F005)
//! - [`content_rules`] - Content-based filtering rules (F006)
//! - [`rule_engine`] - Unified rule evaluation engine (F007)
//! - [`profile`] - User profile management (F019)

pub mod auth;
pub mod classifier;
pub mod content_rules;
pub mod profile;
pub mod rule_engine;
pub mod time_rules;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_engine_can_be_created() {
        let _engine = rule_engine::RuleEngine::new();
    }

    #[test]
    fn rule_engine_with_defaults() {
        let engine = rule_engine::RuleEngine::with_defaults();
        assert!(!engine.time_rules.rules.is_empty());
        assert!(!engine.content_rules.rules.is_empty());
    }

    #[test]
    fn auth_can_be_created() {
        let _auth = auth::AuthManager::new();
    }

    #[test]
    fn auth_can_hash_and_verify_password() {
        let auth = auth::AuthManager::new();
        let hash = auth.hash_password("password123").unwrap();
        assert!(auth.verify_password("password123", &hash).unwrap());
    }

    #[test]
    fn profile_can_be_created() {
        let profile = profile::UserProfile::with_child_defaults("Child", Some("child".to_string()));
        assert_eq!(profile.name, "Child");
        assert!(!profile.time_rules.rules.is_empty());
    }

    #[test]
    fn profile_manager_can_lookup_by_os_username() {
        let mut manager = profile::ProfileManager::new();
        manager.add_profile(profile::UserProfile::with_child_defaults(
            "Child",
            Some("child".to_string()),
        ));

        let found = manager.get_by_os_username("child");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Child");
    }
}

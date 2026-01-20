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
//! - [`community_rules`] - Layered community rules with tier priorities (F025)
//! - [`rule_engine`] - Unified rule evaluation engine (F007)
//! - [`profile`] - User profile management (F019)
//! - [`profile_proxy`] - Profile-aware proxy control with OS user monitoring (F029)
//! - [`protection`] - Protection state toggle with auth-guarded operations (F018)
//! - [`notifications`] - Desktop notifications for blocked content (F014)
//! - [`interception`] - Interception mode switching between extension and proxy (F017)
//! - [`site_registry`] - Dynamic site registry for LLM domain management (F027)
//! - [`extension_install`] - Browser extension auto-installation (F024)

pub mod auth;
pub mod classifier;
pub mod community_rules;
pub mod content_rules;
#[cfg(feature = "extension-install")]
pub mod extension_install;
pub mod interception;
pub mod model_downloader;
pub mod notifications;
pub mod profile;
pub mod profile_proxy;
pub mod protection;
pub mod rule_engine;
pub mod site_registry;
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

    #[test]
    fn protection_manager_can_be_created() {
        let manager = protection::ProtectionManager::new();
        assert_eq!(manager.state(), protection::ProtectionState::Active);
    }

    #[test]
    fn protection_manager_pause_requires_auth() {
        let manager = protection::ProtectionManager::new();
        let auth = auth::AuthManager::new();
        let session = auth.create_session();

        // Pause with valid session
        manager
            .pause(protection::PauseDuration::FIVE_MINUTES, &session, &auth)
            .unwrap();
        assert_eq!(manager.state(), protection::ProtectionState::Paused);

        // Resume
        manager.resume();
        assert_eq!(manager.state(), protection::ProtectionState::Active);
    }

    #[test]
    fn notification_manager_can_be_created() {
        let manager = notifications::NotificationManager::new();
        assert!(manager.is_enabled());
    }

    #[test]
    fn notification_manager_can_be_disabled() {
        let manager = notifications::NotificationManager::new();
        manager.disable();
        assert!(!manager.is_enabled());
    }

    #[test]
    fn notification_manager_respects_disabled_state() {
        let manager = notifications::NotificationManager::with_settings(
            notifications::NotificationSettings::disabled(),
        );
        let event = notifications::BlockedEvent::new(
            Some("Test".to_string()),
            Some(classifier::Category::Violence),
            None,
            false,
        );
        let result = manager.notify_block(&event);
        assert!(result.was_disabled());
    }

    #[test]
    fn interception_manager_can_be_created() {
        let manager = interception::InterceptionManager::new();
        assert_eq!(manager.mode(), interception::InterceptionMode::Extension);
    }

    #[test]
    fn interception_manager_mode_switch_requires_auth() {
        let manager = interception::InterceptionManager::new();
        let auth = auth::AuthManager::new();
        let session = auth.create_session();

        // Switch to proxy mode
        manager
            .set_mode(interception::InterceptionMode::Proxy, &session, &auth)
            .unwrap();
        assert_eq!(manager.mode(), interception::InterceptionMode::Proxy);
        assert!(manager.is_proxy_mode());

        // Switch back to extension mode
        manager
            .set_mode(interception::InterceptionMode::Extension, &session, &auth)
            .unwrap();
        assert_eq!(manager.mode(), interception::InterceptionMode::Extension);
        assert!(manager.is_extension_mode());
    }

    #[test]
    fn site_registry_can_be_created() {
        let registry = site_registry::SiteRegistry::new();
        assert!(registry.exact_sites().is_empty());
    }

    #[test]
    fn site_registry_with_defaults_has_sites() {
        let registry = site_registry::SiteRegistry::with_defaults();
        assert!(registry.is_monitored("api.openai.com"));
        assert!(registry.is_monitored("claude.ai"));
    }

    #[test]
    fn site_registry_service_name() {
        let registry = site_registry::SiteRegistry::with_defaults();
        assert_eq!(registry.service_name("api.openai.com"), "ChatGPT");
        assert_eq!(registry.service_name("claude.ai"), "Claude");
    }
}

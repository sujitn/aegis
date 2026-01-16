//! User profile management (F019).
//!
//! Provides per-child user profiles that map OS usernames to time and content rules.
//!
//! ## Features
//!
//! - Profile with name, OS username, time rules, content rules
//! - Auto-detect OS user to load correct profile
//! - No profile = unrestricted (parent mode)
//! - Default presets for child-safe profiles
//!
//! ## Usage
//!
//! ```
//! use aegis_core::profile::{UserProfile, ProfileManager};
//! use aegis_core::time_rules::TimeRuleSet;
//! use aegis_core::content_rules::ContentRuleSet;
//!
//! // Create a child profile with defaults
//! let profile = UserProfile::with_child_defaults("Child", Some("child_user".to_string()));
//!
//! // Create a profile manager
//! let mut manager = ProfileManager::new();
//! manager.add_profile(profile);
//!
//! // Look up profile by OS username
//! if let Some(profile) = manager.get_by_os_username("child_user") {
//!     println!("Found profile: {}", profile.name);
//! }
//! ```

use serde::{Deserialize, Serialize};

use crate::content_rules::ContentRuleSet;
use crate::time_rules::TimeRuleSet;

/// Proxy behavior mode for a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    /// Proxy is enabled, filtering is active for this profile.
    #[default]
    Enabled,

    /// Proxy is disabled, no filtering for this profile (parent mode).
    Disabled,

    /// Proxy is running but all traffic passes through without filtering.
    Passthrough,
}

impl ProxyMode {
    /// Returns the mode as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::Passthrough => "passthrough",
        }
    }

    /// Returns a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Enabled => "Filtering enabled",
            Self::Disabled => "Filtering disabled (unrestricted)",
            Self::Passthrough => "Proxy active but not filtering",
        }
    }

    /// Returns true if filtering is active.
    pub fn is_filtering(&self) -> bool {
        matches!(self, Self::Enabled)
    }

    /// Returns true if the system proxy should be enabled.
    pub fn needs_system_proxy(&self) -> bool {
        matches!(self, Self::Enabled | Self::Passthrough)
    }
}

impl std::fmt::Display for ProxyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Profile type for categorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProfileType {
    /// Child profile - filtering enabled by default.
    #[default]
    Child,

    /// Parent profile - unrestricted access.
    Parent,
}

impl ProfileType {
    /// Returns the type as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Child => "child",
            Self::Parent => "parent",
        }
    }

    /// Returns true if this is a child profile.
    pub fn is_child(&self) -> bool {
        matches!(self, Self::Child)
    }

    /// Returns true if this is a parent profile.
    pub fn is_parent(&self) -> bool {
        matches!(self, Self::Parent)
    }

    /// Returns the default proxy mode for this profile type.
    pub fn default_proxy_mode(&self) -> ProxyMode {
        match self {
            Self::Child => ProxyMode::Enabled,
            Self::Parent => ProxyMode::Disabled,
        }
    }
}

impl std::fmt::Display for ProfileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A user profile with associated rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// Unique identifier for the profile.
    pub id: String,
    /// Display name for the profile.
    pub name: String,
    /// OS username to auto-match (None = manual selection only).
    pub os_username: Option<String>,
    /// Time-based rules for this profile.
    pub time_rules: TimeRuleSet,
    /// Content-based rules for this profile.
    pub content_rules: ContentRuleSet,
    /// Whether this profile is enabled.
    pub enabled: bool,
    /// Profile type (child or parent).
    #[serde(default)]
    pub profile_type: ProfileType,
    /// Proxy behavior mode for this profile.
    #[serde(default)]
    pub proxy_mode: ProxyMode,
}

impl UserProfile {
    /// Creates a new user profile.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        os_username: Option<String>,
        time_rules: TimeRuleSet,
        content_rules: ContentRuleSet,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            os_username,
            time_rules,
            content_rules,
            enabled: true,
            profile_type: ProfileType::Child,
            proxy_mode: ProxyMode::Enabled,
        }
    }

    /// Creates a profile with child-safe defaults.
    ///
    /// Includes:
    /// - School night and weekend bedtimes
    /// - Family-safe content filtering
    /// - Filtering enabled (child profile type)
    pub fn with_child_defaults(name: impl Into<String>, os_username: Option<String>) -> Self {
        let name = name.into();
        let id = format!("profile_{}", name.to_lowercase().replace([' ', '-'], "_"));

        Self {
            id,
            name,
            os_username,
            time_rules: TimeRuleSet::with_defaults(),
            content_rules: ContentRuleSet::family_safe_defaults(),
            enabled: true,
            profile_type: ProfileType::Child,
            proxy_mode: ProxyMode::Enabled,
        }
    }

    /// Creates an unrestricted profile (parent mode).
    ///
    /// Parent profiles have filtering disabled by default.
    pub fn unrestricted(name: impl Into<String>, os_username: Option<String>) -> Self {
        let name = name.into();
        let id = format!("profile_{}", name.to_lowercase().replace([' ', '-'], "_"));

        Self {
            id,
            name,
            os_username,
            time_rules: TimeRuleSet::new(),
            content_rules: ContentRuleSet::new(),
            enabled: true,
            profile_type: ProfileType::Parent,
            proxy_mode: ProxyMode::Disabled,
        }
    }

    /// Enables this profile.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disables this profile.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Sets the OS username for auto-matching.
    pub fn with_os_username(mut self, os_username: Option<String>) -> Self {
        self.os_username = os_username;
        self
    }

    /// Sets the time rules for this profile.
    pub fn with_time_rules(mut self, time_rules: TimeRuleSet) -> Self {
        self.time_rules = time_rules;
        self
    }

    /// Sets the content rules for this profile.
    pub fn with_content_rules(mut self, content_rules: ContentRuleSet) -> Self {
        self.content_rules = content_rules;
        self
    }

    /// Sets the profile type.
    pub fn with_profile_type(mut self, profile_type: ProfileType) -> Self {
        self.profile_type = profile_type;
        self
    }

    /// Sets the proxy mode.
    pub fn with_proxy_mode(mut self, proxy_mode: ProxyMode) -> Self {
        self.proxy_mode = proxy_mode;
        self
    }

    /// Returns true if this profile requires filtering (child with enabled proxy).
    pub fn requires_filtering(&self) -> bool {
        self.enabled && self.proxy_mode.is_filtering()
    }

    /// Returns true if this profile should have the system proxy enabled.
    pub fn needs_system_proxy(&self) -> bool {
        self.enabled && self.proxy_mode.needs_system_proxy()
    }

    /// Checks if this profile matches the given OS username.
    pub fn matches_os_username(&self, username: &str) -> bool {
        self.enabled
            && self
                .os_username
                .as_ref()
                .map(|u| u.eq_ignore_ascii_case(username))
                .unwrap_or(false)
    }
}

/// Manages a collection of user profiles.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileManager {
    /// All profiles managed by this instance.
    profiles: Vec<UserProfile>,
}

impl ProfileManager {
    /// Creates a new empty profile manager.
    pub fn new() -> Self {
        Self {
            profiles: Vec::new(),
        }
    }

    /// Adds a profile to the manager.
    pub fn add_profile(&mut self, profile: UserProfile) {
        self.profiles.push(profile);
    }

    /// Removes a profile by ID.
    pub fn remove_profile(&mut self, id: &str) -> Option<UserProfile> {
        if let Some(pos) = self.profiles.iter().position(|p| p.id == id) {
            Some(self.profiles.remove(pos))
        } else {
            None
        }
    }

    /// Gets a profile by ID.
    pub fn get_profile(&self, id: &str) -> Option<&UserProfile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// Gets a mutable reference to a profile by ID.
    pub fn get_profile_mut(&mut self, id: &str) -> Option<&mut UserProfile> {
        self.profiles.iter_mut().find(|p| p.id == id)
    }

    /// Gets a profile by OS username (case-insensitive).
    ///
    /// Returns the first enabled profile that matches the given OS username.
    /// Returns None if no profile matches (parent mode / unrestricted).
    pub fn get_by_os_username(&self, username: &str) -> Option<&UserProfile> {
        self.profiles
            .iter()
            .find(|p| p.matches_os_username(username))
    }

    /// Gets a mutable reference to a profile by OS username.
    pub fn get_by_os_username_mut(&mut self, username: &str) -> Option<&mut UserProfile> {
        self.profiles
            .iter_mut()
            .find(|p| p.matches_os_username(username))
    }

    /// Gets all profiles.
    pub fn all_profiles(&self) -> &[UserProfile] {
        &self.profiles
    }

    /// Gets all enabled profiles.
    pub fn enabled_profiles(&self) -> Vec<&UserProfile> {
        self.profiles.iter().filter(|p| p.enabled).collect()
    }

    /// Gets the number of profiles.
    pub fn profile_count(&self) -> usize {
        self.profiles.len()
    }

    /// Checks if a profile exists with the given ID.
    pub fn has_profile(&self, id: &str) -> bool {
        self.profiles.iter().any(|p| p.id == id)
    }

    /// Gets the profile for the current OS user.
    ///
    /// Returns None if no profile matches (parent mode / unrestricted).
    pub fn get_current_profile(&self) -> Option<&UserProfile> {
        let username = get_current_os_user();
        self.get_by_os_username(&username)
    }
}

/// Gets the current OS username.
///
/// Returns the username of the currently logged-in user.
/// This is used to auto-detect which profile to apply.
pub fn get_current_os_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== UserProfile Tests ====================

    #[test]
    fn test_user_profile_new() {
        let profile = UserProfile::new(
            "test_id",
            "Test Profile",
            Some("testuser".to_string()),
            TimeRuleSet::new(),
            ContentRuleSet::new(),
        );

        assert_eq!(profile.id, "test_id");
        assert_eq!(profile.name, "Test Profile");
        assert_eq!(profile.os_username, Some("testuser".to_string()));
        assert!(profile.enabled);
    }

    #[test]
    fn test_user_profile_with_child_defaults() {
        let profile = UserProfile::with_child_defaults("Test Child", Some("child".to_string()));

        assert_eq!(profile.id, "profile_test_child");
        assert_eq!(profile.name, "Test Child");
        assert_eq!(profile.os_username, Some("child".to_string()));
        assert!(profile.enabled);
        // Check that defaults are applied
        assert!(!profile.time_rules.rules.is_empty());
        assert!(!profile.content_rules.rules.is_empty());
    }

    #[test]
    fn test_user_profile_unrestricted() {
        let profile = UserProfile::unrestricted("Parent", None);

        assert_eq!(profile.name, "Parent");
        assert!(profile.enabled);
        // Unrestricted = no rules
        assert!(profile.time_rules.rules.is_empty());
        assert!(profile.content_rules.rules.is_empty());
    }

    #[test]
    fn test_user_profile_enable_disable() {
        let mut profile = UserProfile::with_child_defaults("Child", None);

        assert!(profile.enabled);
        profile.disable();
        assert!(!profile.enabled);
        profile.enable();
        assert!(profile.enabled);
    }

    #[test]
    fn test_user_profile_matches_os_username() {
        let profile = UserProfile::with_child_defaults("Child", Some("childuser".to_string()));

        assert!(profile.matches_os_username("childuser"));
        assert!(profile.matches_os_username("CHILDUSER")); // Case insensitive
        assert!(profile.matches_os_username("ChildUser")); // Case insensitive
        assert!(!profile.matches_os_username("other"));
    }

    #[test]
    fn test_user_profile_matches_os_username_disabled() {
        let mut profile = UserProfile::with_child_defaults("Child", Some("childuser".to_string()));
        profile.disable();

        // Disabled profile should not match
        assert!(!profile.matches_os_username("childuser"));
    }

    #[test]
    fn test_user_profile_matches_no_os_username() {
        let profile = UserProfile::with_child_defaults("Child", None);

        // Profile without os_username should not match any
        assert!(!profile.matches_os_username("anyuser"));
    }

    #[test]
    fn test_user_profile_builder_pattern() {
        let profile = UserProfile::unrestricted("Parent", None)
            .with_os_username(Some("parent".to_string()))
            .with_time_rules(TimeRuleSet::with_defaults())
            .with_content_rules(ContentRuleSet::permissive_defaults());

        assert_eq!(profile.os_username, Some("parent".to_string()));
        assert!(!profile.time_rules.rules.is_empty());
        assert!(!profile.content_rules.rules.is_empty());
    }

    // ==================== ProfileManager Tests ====================

    #[test]
    fn test_profile_manager_new() {
        let manager = ProfileManager::new();
        assert_eq!(manager.profile_count(), 0);
    }

    #[test]
    fn test_profile_manager_add_and_get() {
        let mut manager = ProfileManager::new();
        let profile = UserProfile::with_child_defaults("Child", Some("child".to_string()));

        manager.add_profile(profile);

        assert_eq!(manager.profile_count(), 1);
        assert!(manager.get_profile("profile_child").is_some());
    }

    #[test]
    fn test_profile_manager_remove() {
        let mut manager = ProfileManager::new();
        manager.add_profile(UserProfile::with_child_defaults(
            "Child",
            Some("child".to_string()),
        ));

        let removed = manager.remove_profile("profile_child");
        assert!(removed.is_some());
        assert_eq!(manager.profile_count(), 0);

        // Remove non-existent
        let removed = manager.remove_profile("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn test_profile_manager_get_by_os_username() {
        let mut manager = ProfileManager::new();
        manager.add_profile(UserProfile::with_child_defaults(
            "Alice",
            Some("alice".to_string()),
        ));
        manager.add_profile(UserProfile::with_child_defaults(
            "Bob",
            Some("bob".to_string()),
        ));

        let alice = manager.get_by_os_username("alice");
        assert!(alice.is_some());
        assert_eq!(alice.unwrap().name, "Alice");

        let bob = manager.get_by_os_username("BOB"); // Case insensitive
        assert!(bob.is_some());
        assert_eq!(bob.unwrap().name, "Bob");

        // No match = parent mode
        let parent = manager.get_by_os_username("parentuser");
        assert!(parent.is_none());
    }

    #[test]
    fn test_profile_manager_enabled_profiles() {
        let mut manager = ProfileManager::new();

        let mut disabled_profile =
            UserProfile::with_child_defaults("Disabled", Some("disabled".to_string()));
        disabled_profile.disable();

        manager.add_profile(UserProfile::with_child_defaults(
            "Enabled",
            Some("enabled".to_string()),
        ));
        manager.add_profile(disabled_profile);

        let enabled = manager.enabled_profiles();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "Enabled");
    }

    #[test]
    fn test_profile_manager_has_profile() {
        let mut manager = ProfileManager::new();
        manager.add_profile(UserProfile::with_child_defaults("Child", None));

        assert!(manager.has_profile("profile_child"));
        assert!(!manager.has_profile("nonexistent"));
    }

    #[test]
    fn test_profile_manager_all_profiles() {
        let mut manager = ProfileManager::new();
        manager.add_profile(UserProfile::with_child_defaults("One", None));
        manager.add_profile(UserProfile::with_child_defaults("Two", None));

        let all = manager.all_profiles();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_profile_manager_get_profile_mut() {
        let mut manager = ProfileManager::new();
        manager.add_profile(UserProfile::with_child_defaults("Child", None));

        if let Some(profile) = manager.get_profile_mut("profile_child") {
            profile.name = "Updated Child".to_string();
        }

        assert_eq!(
            manager.get_profile("profile_child").unwrap().name,
            "Updated Child"
        );
    }

    // ==================== get_current_os_user Tests ====================

    #[test]
    fn test_get_current_os_user() {
        let username = get_current_os_user();
        // Should return something (either USER, USERNAME, or "unknown")
        assert!(!username.is_empty());
    }

    // ==================== Serialization Tests ====================

    #[test]
    fn test_user_profile_serialization() {
        let profile = UserProfile::with_child_defaults("Child", Some("child".to_string()));
        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: UserProfile = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, profile.id);
        assert_eq!(deserialized.name, profile.name);
        assert_eq!(deserialized.os_username, profile.os_username);
    }

    #[test]
    fn test_profile_manager_serialization() {
        let mut manager = ProfileManager::new();
        manager.add_profile(UserProfile::with_child_defaults(
            "Child",
            Some("child".to_string()),
        ));

        let json = serde_json::to_string(&manager).unwrap();
        let deserialized: ProfileManager = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.profile_count(), 1);
    }
}

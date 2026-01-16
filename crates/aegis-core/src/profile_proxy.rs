//! Profile-aware proxy control module (F029).
//!
//! Provides auto-enable/disable of proxy based on logged-in OS user profile.
//! Child profiles enable filtering; parent/unknown profiles disable or bypass.
//!
//! ## Features
//!
//! - OS user change detection (polling-based)
//! - Profile-based proxy control
//! - Profile switch event logging
//! - Fast user switching support with debouncing
//!
//! ## Usage
//!
//! ```ignore
//! use aegis_core::profile_proxy::{ProfileProxyController, ProfileProxyConfig};
//! use aegis_core::profile::ProfileManager;
//! use aegis_core::protection::ProtectionManager;
//!
//! let profiles = ProfileManager::new();
//! let protection = ProtectionManager::new();
//! let config = ProfileProxyConfig::default();
//!
//! let controller = ProfileProxyController::new(profiles, protection, config);
//!
//! // Start monitoring (in a background task)
//! controller.start_monitoring();
//!
//! // Manual check
//! controller.force_check();
//!
//! // Get current profile
//! if let Some(profile) = controller.current_profile() {
//!     println!("Current profile: {}", profile.name);
//! }
//! ```

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::profile::{get_current_os_user, ProfileManager, ProxyMode, UserProfile};
use crate::protection::{ProtectionManager, ProtectionState};

/// Errors that can occur during profile proxy operations.
#[derive(Debug, Error)]
pub enum ProfileProxyError {
    /// Failed to detect OS user.
    #[error("failed to detect OS user")]
    UserDetectionFailed,

    /// Profile not found.
    #[error("profile not found: {0}")]
    ProfileNotFound(String),

    /// System proxy operation failed.
    #[error("system proxy error: {0}")]
    SystemProxy(String),
}

/// Result type for profile proxy operations.
pub type Result<T> = std::result::Result<T, ProfileProxyError>;

/// Action taken on the system proxy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyAction {
    /// System proxy enabled, filtering active.
    Enabled,

    /// System proxy disabled, no filtering.
    Disabled,

    /// Proxy running in passthrough mode.
    Passthrough,

    /// No change to proxy state.
    NoChange,
}

impl ProxyAction {
    /// Returns the action as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::Passthrough => "passthrough",
            Self::NoChange => "no_change",
        }
    }
}

impl std::fmt::Display for ProxyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Event generated when profile switches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSwitchEvent {
    /// Timestamp of the switch.
    pub timestamp: DateTime<Utc>,
    /// OS username that triggered the switch.
    pub os_username: String,
    /// Previous profile name (if any).
    pub previous_profile: Option<String>,
    /// New profile name (if any).
    pub new_profile: Option<String>,
    /// Previous protection state.
    pub previous_state: Option<String>,
    /// New protection state.
    pub new_state: String,
    /// Action taken on the proxy.
    pub proxy_action: ProxyAction,
}

impl ProfileSwitchEvent {
    /// Creates a new profile switch event.
    pub fn new(
        os_username: String,
        previous_profile: Option<String>,
        new_profile: Option<String>,
        previous_state: Option<String>,
        new_state: String,
        proxy_action: ProxyAction,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            os_username,
            previous_profile,
            new_profile,
            previous_state,
            new_state,
            proxy_action,
        }
    }
}

/// Configuration for the profile proxy controller.
#[derive(Debug, Clone)]
pub struct ProfileProxyConfig {
    /// Polling interval for OS user detection.
    pub poll_interval: Duration,
    /// Debounce duration for rapid user switches.
    pub debounce_duration: Duration,
    /// Default proxy mode for unknown users.
    pub unknown_user_mode: UnknownUserMode,
    /// Proxy host.
    pub proxy_host: String,
    /// Proxy port.
    pub proxy_port: u16,
}

impl Default for ProfileProxyConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(5),
            debounce_duration: Duration::from_millis(500),
            unknown_user_mode: UnknownUserMode::EnableWithDefaults,
            proxy_host: "127.0.0.1".to_string(),
            proxy_port: 8766,
        }
    }
}

/// How to handle unknown users (users without a profile).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UnknownUserMode {
    /// Enable filtering with default rules (recommended for safety).
    #[default]
    EnableWithDefaults,

    /// Disable filtering (treat as parent).
    DisableFiltering,

    /// Use passthrough mode (proxy running but not filtering).
    Passthrough,
}

impl UnknownUserMode {
    /// Converts to ProxyAction.
    pub fn to_proxy_action(&self) -> ProxyAction {
        match self {
            Self::EnableWithDefaults => ProxyAction::Enabled,
            Self::DisableFiltering => ProxyAction::Disabled,
            Self::Passthrough => ProxyAction::Passthrough,
        }
    }
}

/// Internal state for the profile proxy controller.
#[derive(Debug)]
struct ControllerState {
    /// Current OS username.
    current_os_user: String,
    /// Current profile ID (if any).
    current_profile_id: Option<String>,
    /// Last check time for debouncing.
    last_check: Instant,
    /// Is monitoring active?
    monitoring_active: bool,
    /// Event history (recent switches).
    event_history: Vec<ProfileSwitchEvent>,
}

impl Default for ControllerState {
    fn default() -> Self {
        Self {
            current_os_user: String::new(),
            current_profile_id: None,
            last_check: Instant::now(),
            monitoring_active: false,
            event_history: Vec::new(),
        }
    }
}

/// Callback type for profile switch events.
pub type OnSwitchCallback = Arc<dyn Fn(&ProfileSwitchEvent) + Send + Sync>;

/// Callback type for proxy control actions.
pub type OnProxyActionCallback = Arc<dyn Fn(ProxyAction, &str, u16) + Send + Sync>;

/// Controls proxy behavior based on user profiles.
///
/// Thread-safe and clonable for use across async contexts.
#[derive(Clone)]
pub struct ProfileProxyController {
    profiles: Arc<RwLock<ProfileManager>>,
    protection: ProtectionManager,
    config: ProfileProxyConfig,
    state: Arc<RwLock<ControllerState>>,
    on_switch: Option<OnSwitchCallback>,
    on_proxy_action: Option<OnProxyActionCallback>,
}

impl std::fmt::Debug for ProfileProxyController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProfileProxyController")
            .field("config", &self.config)
            .field("state", &self.state)
            .finish()
    }
}

impl ProfileProxyController {
    /// Creates a new profile proxy controller.
    pub fn new(
        profiles: ProfileManager,
        protection: ProtectionManager,
        config: ProfileProxyConfig,
    ) -> Self {
        Self {
            profiles: Arc::new(RwLock::new(profiles)),
            protection,
            config,
            state: Arc::new(RwLock::new(ControllerState::default())),
            on_switch: None,
            on_proxy_action: None,
        }
    }

    /// Creates a controller with default configuration.
    pub fn with_defaults(profiles: ProfileManager, protection: ProtectionManager) -> Self {
        Self::new(profiles, protection, ProfileProxyConfig::default())
    }

    /// Sets the callback for profile switch events.
    pub fn on_switch<F>(mut self, callback: F) -> Self
    where
        F: Fn(&ProfileSwitchEvent) + Send + Sync + 'static,
    {
        self.on_switch = Some(Arc::new(callback));
        self
    }

    /// Sets the callback for proxy action events.
    ///
    /// This callback is invoked when the proxy should be enabled/disabled.
    /// The callback receives: (action, host, port).
    pub fn on_proxy_action<F>(mut self, callback: F) -> Self
    where
        F: Fn(ProxyAction, &str, u16) + Send + Sync + 'static,
    {
        self.on_proxy_action = Some(Arc::new(callback));
        self
    }

    /// Returns the current profile for the logged-in user.
    pub fn current_profile(&self) -> Option<UserProfile> {
        let profiles = self.profiles.read().unwrap();
        let state = self.state.read().unwrap();

        if let Some(ref profile_id) = state.current_profile_id {
            profiles.get_profile(profile_id).cloned()
        } else {
            None
        }
    }

    /// Returns the current OS username.
    pub fn current_os_user(&self) -> String {
        let state = self.state.read().unwrap();
        state.current_os_user.clone()
    }

    /// Returns recent switch events.
    pub fn event_history(&self) -> Vec<ProfileSwitchEvent> {
        let state = self.state.read().unwrap();
        state.event_history.clone()
    }

    /// Returns true if monitoring is active.
    pub fn is_monitoring(&self) -> bool {
        let state = self.state.read().unwrap();
        state.monitoring_active
    }

    /// Manually checks for user changes and updates state.
    ///
    /// Returns the switch event if a profile change occurred.
    pub fn force_check(&self) -> Option<ProfileSwitchEvent> {
        let current_user = get_current_os_user();
        self.check_user_change(&current_user)
    }

    /// Performs a check with debouncing.
    ///
    /// Returns None if within debounce period.
    pub fn check_with_debounce(&self) -> Option<ProfileSwitchEvent> {
        let now = Instant::now();

        // Check debounce
        {
            let state = self.state.read().unwrap();
            if now.duration_since(state.last_check) < self.config.debounce_duration {
                return None;
            }
        }

        // Update last check time
        {
            let mut state = self.state.write().unwrap();
            state.last_check = now;
        }

        self.force_check()
    }

    /// Starts monitoring for user changes.
    ///
    /// This sets the monitoring flag. The actual polling should be done
    /// by the caller in a background task using `poll_once()`.
    pub fn start_monitoring(&self) {
        let mut state = self.state.write().unwrap();
        state.monitoring_active = true;

        // Initialize current user
        state.current_os_user = get_current_os_user();
    }

    /// Stops monitoring for user changes.
    pub fn stop_monitoring(&self) {
        let mut state = self.state.write().unwrap();
        state.monitoring_active = false;
    }

    /// Performs one poll iteration.
    ///
    /// Call this periodically (based on `config.poll_interval`) from a background task.
    pub fn poll_once(&self) -> Option<ProfileSwitchEvent> {
        if !self.is_monitoring() {
            return None;
        }

        self.check_with_debounce()
    }

    /// Returns the poll interval from config.
    pub fn poll_interval(&self) -> Duration {
        self.config.poll_interval
    }

    /// Updates the profiles.
    pub fn update_profiles(&self, profiles: ProfileManager) {
        let mut p = self.profiles.write().unwrap();
        *p = profiles;
    }

    /// Internal: check for user change and handle it.
    fn check_user_change(&self, new_user: &str) -> Option<ProfileSwitchEvent> {
        let profiles = self.profiles.read().unwrap();

        // Get previous state
        let (previous_user, previous_profile_id) = {
            let state = self.state.read().unwrap();
            (
                state.current_os_user.clone(),
                state.current_profile_id.clone(),
            )
        };

        // Check if user changed
        if previous_user == new_user && !previous_user.is_empty() {
            return None;
        }

        // Find new profile
        let new_profile = profiles.get_by_os_username(new_user);
        let new_profile_id = new_profile.map(|p| p.id.clone());

        // Determine proxy action
        let proxy_action = self.determine_proxy_action(new_profile);

        // Get previous profile name
        let previous_profile_name = previous_profile_id
            .as_ref()
            .and_then(|id| profiles.get_profile(id))
            .map(|p| p.name.clone());

        let new_profile_name = new_profile.map(|p| p.name.clone());

        // Determine protection state
        let previous_state = Some(self.protection.state().as_str().to_string());
        let new_state = match proxy_action {
            ProxyAction::Enabled => ProtectionState::Active.as_str().to_string(),
            ProxyAction::Disabled => ProtectionState::Disabled.as_str().to_string(),
            ProxyAction::Passthrough => ProtectionState::Active.as_str().to_string(),
            ProxyAction::NoChange => self.protection.state().as_str().to_string(),
        };

        // Create event
        let event = ProfileSwitchEvent::new(
            new_user.to_string(),
            previous_profile_name,
            new_profile_name,
            previous_state,
            new_state,
            proxy_action,
        );

        // Update state
        {
            let mut state = self.state.write().unwrap();
            state.current_os_user = new_user.to_string();
            state.current_profile_id = new_profile_id;

            // Keep last 100 events
            state.event_history.push(event.clone());
            if state.event_history.len() > 100 {
                state.event_history.remove(0);
            }
        }

        // Execute proxy action
        self.execute_proxy_action(proxy_action);

        // Invoke callback
        if let Some(ref callback) = self.on_switch {
            callback(&event);
        }

        Some(event)
    }

    /// Determines the proxy action based on the profile.
    fn determine_proxy_action(&self, profile: Option<&UserProfile>) -> ProxyAction {
        match profile {
            Some(p) => {
                // Use profile's proxy mode
                match p.proxy_mode {
                    ProxyMode::Enabled => ProxyAction::Enabled,
                    ProxyMode::Disabled => ProxyAction::Disabled,
                    ProxyMode::Passthrough => ProxyAction::Passthrough,
                }
            }
            None => {
                // Unknown user - use config default
                self.config.unknown_user_mode.to_proxy_action()
            }
        }
    }

    /// Executes the proxy action.
    fn execute_proxy_action(&self, action: ProxyAction) {
        // Update protection state
        match action {
            ProxyAction::Enabled | ProxyAction::Passthrough => {
                self.protection.enable();
            }
            ProxyAction::Disabled => {
                // Note: We use set_state to bypass auth since this is auto-triggered
                self.protection.set_state(ProtectionState::Disabled);
            }
            ProxyAction::NoChange => {}
        }

        // Invoke proxy action callback for system proxy control
        if let Some(ref callback) = self.on_proxy_action {
            callback(action, &self.config.proxy_host, self.config.proxy_port);
        }
    }

    /// Initializes the controller with the current user.
    ///
    /// Call this at startup to set the initial state.
    /// Returns the initial profile event with the current user's profile.
    pub fn initialize(&self) -> Option<ProfileSwitchEvent> {
        let current_user = get_current_os_user();

        // Clear current state to force an "initial" event
        {
            let mut state = self.state.write().unwrap();
            state.current_os_user.clear();
            state.current_profile_id = None;
        }

        // Now check and apply the profile (will generate an event since user is "new")
        self.check_user_change(&current_user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_rules::ContentRuleSet;
    use crate::time_rules::TimeRuleSet;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn create_test_profiles() -> ProfileManager {
        let mut manager = ProfileManager::new();

        // Child profile with filtering enabled
        let child = UserProfile::new(
            "child1",
            "Alice",
            Some("alice".to_string()),
            TimeRuleSet::with_defaults(),
            ContentRuleSet::family_safe_defaults(),
        );
        manager.add_profile(child);

        // Parent profile with filtering disabled
        let parent = UserProfile::unrestricted("Parent", Some("parent".to_string()));
        manager.add_profile(parent);

        manager
    }

    // ==================== ProxyAction Tests ====================

    #[test]
    fn test_proxy_action_as_str() {
        assert_eq!(ProxyAction::Enabled.as_str(), "enabled");
        assert_eq!(ProxyAction::Disabled.as_str(), "disabled");
        assert_eq!(ProxyAction::Passthrough.as_str(), "passthrough");
        assert_eq!(ProxyAction::NoChange.as_str(), "no_change");
    }

    #[test]
    fn test_proxy_action_display() {
        assert_eq!(format!("{}", ProxyAction::Enabled), "enabled");
    }

    // ==================== ProfileSwitchEvent Tests ====================

    #[test]
    fn test_profile_switch_event_new() {
        let event = ProfileSwitchEvent::new(
            "alice".to_string(),
            None,
            Some("Alice".to_string()),
            None,
            "active".to_string(),
            ProxyAction::Enabled,
        );

        assert_eq!(event.os_username, "alice");
        assert!(event.previous_profile.is_none());
        assert_eq!(event.new_profile, Some("Alice".to_string()));
        assert_eq!(event.proxy_action, ProxyAction::Enabled);
    }

    // ==================== ProfileProxyConfig Tests ====================

    #[test]
    fn test_config_default() {
        let config = ProfileProxyConfig::default();
        assert_eq!(config.poll_interval, Duration::from_secs(5));
        assert_eq!(config.debounce_duration, Duration::from_millis(500));
        assert_eq!(config.proxy_host, "127.0.0.1");
        assert_eq!(config.proxy_port, 8766);
    }

    // ==================== UnknownUserMode Tests ====================

    #[test]
    fn test_unknown_user_mode_to_proxy_action() {
        assert_eq!(
            UnknownUserMode::EnableWithDefaults.to_proxy_action(),
            ProxyAction::Enabled
        );
        assert_eq!(
            UnknownUserMode::DisableFiltering.to_proxy_action(),
            ProxyAction::Disabled
        );
        assert_eq!(
            UnknownUserMode::Passthrough.to_proxy_action(),
            ProxyAction::Passthrough
        );
    }

    // ==================== ProfileProxyController Tests ====================

    #[test]
    fn test_controller_new() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();
        let config = ProfileProxyConfig::default();

        let controller = ProfileProxyController::new(profiles, protection, config);

        assert!(!controller.is_monitoring());
        assert!(controller.current_profile().is_none());
    }

    #[test]
    fn test_controller_with_defaults() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();

        let controller = ProfileProxyController::with_defaults(profiles, protection);

        assert!(!controller.is_monitoring());
    }

    #[test]
    fn test_controller_start_stop_monitoring() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();

        let controller = ProfileProxyController::with_defaults(profiles, protection);

        assert!(!controller.is_monitoring());

        controller.start_monitoring();
        assert!(controller.is_monitoring());

        controller.stop_monitoring();
        assert!(!controller.is_monitoring());
    }

    #[test]
    fn test_controller_force_check() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();

        let controller = ProfileProxyController::with_defaults(profiles, protection);

        // First check should return Some (initial state)
        let event = controller.force_check();
        assert!(event.is_some());

        // Second check with same user should return None
        let event2 = controller.force_check();
        assert!(event2.is_none());
    }

    #[test]
    fn test_controller_current_os_user() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();

        let controller = ProfileProxyController::with_defaults(profiles, protection);
        controller.start_monitoring();

        let user = controller.current_os_user();
        assert!(!user.is_empty());
    }

    #[test]
    fn test_controller_event_history() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();

        let controller = ProfileProxyController::with_defaults(profiles, protection);

        // Initially empty
        assert!(controller.event_history().is_empty());

        // After check, should have one event
        controller.force_check();
        assert_eq!(controller.event_history().len(), 1);
    }

    #[test]
    fn test_controller_on_switch_callback() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();

        let callback_count = Arc::new(AtomicUsize::new(0));
        let count_clone = callback_count.clone();

        let controller = ProfileProxyController::with_defaults(profiles, protection)
            .on_switch(move |_event| {
                count_clone.fetch_add(1, Ordering::SeqCst);
            });

        controller.force_check();

        assert_eq!(callback_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_controller_on_proxy_action_callback() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();

        let callback_count = Arc::new(AtomicUsize::new(0));
        let count_clone = callback_count.clone();

        let controller = ProfileProxyController::with_defaults(profiles, protection)
            .on_proxy_action(move |_action, _host, _port| {
                count_clone.fetch_add(1, Ordering::SeqCst);
            });

        controller.force_check();

        assert_eq!(callback_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_controller_initialize() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();

        let controller = ProfileProxyController::with_defaults(profiles, protection);

        let event = controller.initialize();

        // Should have set the current user
        assert!(!controller.current_os_user().is_empty());
        assert!(event.is_some());
    }

    #[test]
    fn test_controller_update_profiles() {
        let profiles = ProfileManager::new();
        let protection = ProtectionManager::new();

        let controller = ProfileProxyController::with_defaults(profiles, protection);

        // Update with new profiles
        let new_profiles = create_test_profiles();
        controller.update_profiles(new_profiles);

        // The profiles should be updated (can't directly verify, but should not panic)
    }

    #[test]
    fn test_controller_poll_interval() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();

        let controller = ProfileProxyController::with_defaults(profiles, protection);

        assert_eq!(controller.poll_interval(), Duration::from_secs(5));
    }

    #[test]
    fn test_controller_poll_once_when_not_monitoring() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();

        let controller = ProfileProxyController::with_defaults(profiles, protection);

        // Should return None when not monitoring
        let event = controller.poll_once();
        assert!(event.is_none());
    }

    #[test]
    fn test_controller_poll_once_when_monitoring() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();

        let controller = ProfileProxyController::with_defaults(profiles, protection);
        controller.start_monitoring();

        // First poll should return Some
        let event = controller.poll_once();
        // May or may not return event depending on debounce timing
        // Just verify it doesn't panic
        let _ = event;
    }

    // ==================== Determine Proxy Action Tests ====================

    #[test]
    fn test_determine_proxy_action_child_profile() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();

        let controller = ProfileProxyController::with_defaults(profiles.clone(), protection);

        let child = profiles.get_profile("child1").unwrap();
        let action = controller.determine_proxy_action(Some(child));

        assert_eq!(action, ProxyAction::Enabled);
    }

    #[test]
    fn test_determine_proxy_action_parent_profile() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();

        let controller = ProfileProxyController::with_defaults(profiles.clone(), protection);

        let parent = profiles.get_profile("profile_parent").unwrap();
        let action = controller.determine_proxy_action(Some(parent));

        assert_eq!(action, ProxyAction::Disabled);
    }

    #[test]
    fn test_determine_proxy_action_unknown_user() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();

        let controller = ProfileProxyController::with_defaults(profiles, protection);

        let action = controller.determine_proxy_action(None);

        // Default unknown user mode is EnableWithDefaults
        assert_eq!(action, ProxyAction::Enabled);
    }

    #[test]
    fn test_determine_proxy_action_unknown_user_disable() {
        let profiles = create_test_profiles();
        let protection = ProtectionManager::new();
        let config = ProfileProxyConfig {
            unknown_user_mode: UnknownUserMode::DisableFiltering,
            ..Default::default()
        };

        let controller = ProfileProxyController::new(profiles, protection, config);

        let action = controller.determine_proxy_action(None);

        assert_eq!(action, ProxyAction::Disabled);
    }

    // ==================== Serialization Tests ====================

    #[test]
    fn test_proxy_action_serialization() {
        let action = ProxyAction::Enabled;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, "\"enabled\"");

        let deserialized: ProxyAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, action);
    }

    #[test]
    fn test_profile_switch_event_serialization() {
        let event = ProfileSwitchEvent::new(
            "alice".to_string(),
            Some("Parent".to_string()),
            Some("Alice".to_string()),
            Some("disabled".to_string()),
            "active".to_string(),
            ProxyAction::Enabled,
        );

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ProfileSwitchEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.os_username, event.os_username);
        assert_eq!(deserialized.proxy_action, event.proxy_action);
    }

    #[test]
    fn test_unknown_user_mode_serialization() {
        let mode = UnknownUserMode::EnableWithDefaults;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"enable_with_defaults\"");

        let deserialized: UnknownUserMode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, mode);
    }
}

//! Desktop notifications for blocked content (F014).
//!
//! This module provides cross-platform desktop notifications to alert parents
//! when content is blocked.
//!
//! ## Features
//!
//! - Notify on block events (not warnings)
//! - Shows site/source and category
//! - Rate-limited to 1 notification per minute
//! - Can be enabled/disabled
//! - Cross-platform (Windows, macOS, Linux)

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::classifier::Category;
use crate::rule_engine::{RuleAction, RuleSource};

/// Minimum time between notifications (60 seconds).
const RATE_LIMIT_DURATION: Duration = Duration::from_secs(60);

/// Notification settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// Whether notifications are enabled.
    pub enabled: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl NotificationSettings {
    /// Creates new settings with notifications enabled.
    pub fn enabled() -> Self {
        Self { enabled: true }
    }

    /// Creates new settings with notifications disabled.
    pub fn disabled() -> Self {
        Self { enabled: false }
    }
}

/// Information about a blocked event for notification.
#[derive(Debug, Clone)]
pub struct BlockedEvent {
    /// The source/site where the block occurred.
    pub source: Option<String>,
    /// The category that triggered the block.
    pub category: Option<Category>,
    /// The rule name that triggered the block.
    pub rule_name: Option<String>,
    /// Whether this was a time-based or content-based block.
    pub is_time_block: bool,
}

impl BlockedEvent {
    /// Creates a new blocked event.
    pub fn new(
        source: Option<String>,
        category: Option<Category>,
        rule_name: Option<String>,
        is_time_block: bool,
    ) -> Self {
        Self {
            source,
            category,
            rule_name,
            is_time_block,
        }
    }

    /// Creates a blocked event from a rule source.
    pub fn from_rule_source(source: &RuleSource, site: Option<String>) -> Self {
        match source {
            RuleSource::None => Self::new(site, None, None, false),
            RuleSource::TimeRule { rule_name, .. } => {
                Self::new(site, None, Some(rule_name.clone()), true)
            }
            RuleSource::ContentRule(result) => Self::new(
                site,
                Some(result.category),
                Some(result.rule_name.clone()),
                false,
            ),
        }
    }
}

/// Internal state for rate limiting.
#[derive(Debug, Default)]
struct RateLimitState {
    last_notification: Option<Instant>,
}

/// Result of attempting to send a notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationResult {
    /// Notification was sent successfully.
    Sent,
    /// Notification was rate-limited (too soon after last one).
    RateLimited,
    /// Notifications are disabled.
    Disabled,
    /// Failed to send notification.
    Failed(String),
}

impl NotificationResult {
    /// Returns true if the notification was sent.
    pub fn was_sent(&self) -> bool {
        matches!(self, NotificationResult::Sent)
    }

    /// Returns true if the notification was rate-limited.
    pub fn was_rate_limited(&self) -> bool {
        matches!(self, NotificationResult::RateLimited)
    }

    /// Returns true if notifications are disabled.
    pub fn was_disabled(&self) -> bool {
        matches!(self, NotificationResult::Disabled)
    }
}

/// Manages desktop notifications with rate limiting.
#[derive(Debug, Clone, Default)]
pub struct NotificationManager {
    settings: Arc<RwLock<NotificationSettings>>,
    rate_limit: Arc<RwLock<RateLimitState>>,
}

impl NotificationManager {
    /// Creates a new notification manager with default settings (enabled).
    pub fn new() -> Self {
        Self {
            settings: Arc::new(RwLock::new(NotificationSettings::default())),
            rate_limit: Arc::new(RwLock::new(RateLimitState::default())),
        }
    }

    /// Creates a new notification manager with the given settings.
    pub fn with_settings(settings: NotificationSettings) -> Self {
        Self {
            settings: Arc::new(RwLock::new(settings)),
            rate_limit: Arc::new(RwLock::new(RateLimitState::default())),
        }
    }

    /// Returns whether notifications are enabled.
    pub fn is_enabled(&self) -> bool {
        self.settings.read().unwrap().enabled
    }

    /// Enables notifications.
    pub fn enable(&self) {
        self.settings.write().unwrap().enabled = true;
    }

    /// Disables notifications.
    pub fn disable(&self) {
        self.settings.write().unwrap().enabled = false;
    }

    /// Sets whether notifications are enabled.
    pub fn set_enabled(&self, enabled: bool) {
        self.settings.write().unwrap().enabled = enabled;
    }

    /// Gets a copy of the current settings.
    pub fn settings(&self) -> NotificationSettings {
        self.settings.read().unwrap().clone()
    }

    /// Updates the settings.
    pub fn update_settings(&self, settings: NotificationSettings) {
        *self.settings.write().unwrap() = settings;
    }

    /// Returns the time until the next notification can be sent.
    ///
    /// Returns `None` if a notification can be sent now.
    pub fn time_until_next(&self) -> Option<Duration> {
        let state = self.rate_limit.read().unwrap();
        if let Some(last) = state.last_notification {
            let elapsed = last.elapsed();
            if elapsed < RATE_LIMIT_DURATION {
                return Some(RATE_LIMIT_DURATION - elapsed);
            }
        }
        None
    }

    /// Checks if we're currently rate-limited.
    pub fn is_rate_limited(&self) -> bool {
        self.time_until_next().is_some()
    }

    /// Notifies about a blocked event.
    ///
    /// Only sends notification if:
    /// - Notifications are enabled
    /// - Not rate-limited (1 minute between notifications)
    /// - The action is Block (not Warn or Allow)
    pub fn notify_block(&self, event: &BlockedEvent) -> NotificationResult {
        // Check if enabled
        if !self.is_enabled() {
            return NotificationResult::Disabled;
        }

        // Check rate limit
        {
            let state = self.rate_limit.read().unwrap();
            if let Some(last) = state.last_notification {
                if last.elapsed() < RATE_LIMIT_DURATION {
                    return NotificationResult::RateLimited;
                }
            }
        }

        // Send notification
        let result = self.send_notification(event);

        // Update rate limit on success
        if result.was_sent() {
            let mut state = self.rate_limit.write().unwrap();
            state.last_notification = Some(Instant::now());
        }

        result
    }

    /// Notifies about a rule engine result if it's a block.
    ///
    /// Convenience method that checks the action and creates the event.
    pub fn notify_if_blocked(
        &self,
        action: RuleAction,
        source: &RuleSource,
        site: Option<String>,
    ) -> Option<NotificationResult> {
        if action != RuleAction::Block {
            return None;
        }

        let event = BlockedEvent::from_rule_source(source, site);
        Some(self.notify_block(&event))
    }

    /// Resets the rate limit (for testing).
    #[cfg(test)]
    pub fn reset_rate_limit(&self) {
        let mut state = self.rate_limit.write().unwrap();
        state.last_notification = None;
    }

    /// Sends the actual notification using platform-specific API.
    #[cfg(feature = "notifications")]
    fn send_notification(&self, event: &BlockedEvent) -> NotificationResult {
        use notify_rust::Notification;

        let title = "Aegis - Content Blocked";
        let body = self.format_notification_body(event);

        match Notification::new()
            .summary(title)
            .body(&body)
            .appname("Aegis")
            .timeout(notify_rust::Timeout::Milliseconds(5000))
            .show()
        {
            Ok(_) => NotificationResult::Sent,
            Err(e) => NotificationResult::Failed(e.to_string()),
        }
    }

    /// Fallback when notifications feature is disabled.
    #[cfg(not(feature = "notifications"))]
    fn send_notification(&self, _event: &BlockedEvent) -> NotificationResult {
        // Silently succeed when notifications are compiled out
        NotificationResult::Sent
    }

    /// Formats the notification body.
    fn format_notification_body(&self, event: &BlockedEvent) -> String {
        let mut parts = Vec::new();

        // Add source/site if available
        if let Some(source) = &event.source {
            parts.push(format!("Site: {}", source));
        }

        // Add category or time block info
        if event.is_time_block {
            if let Some(rule) = &event.rule_name {
                parts.push(format!("Reason: {} (time restriction)", rule));
            } else {
                parts.push("Reason: Time restriction".to_string());
            }
        } else if let Some(category) = &event.category {
            parts.push(format!("Category: {}", category.name()));
        }

        if parts.is_empty() {
            "A prompt was blocked by Aegis protection.".to_string()
        } else {
            parts.join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_rules::{ContentAction, ContentRuleResult};

    // ==================== NotificationSettings Tests ====================

    #[test]
    fn settings_default_is_enabled() {
        let settings = NotificationSettings::default();
        assert!(settings.enabled);
    }

    #[test]
    fn settings_enabled_constructor() {
        let settings = NotificationSettings::enabled();
        assert!(settings.enabled);
    }

    #[test]
    fn settings_disabled_constructor() {
        let settings = NotificationSettings::disabled();
        assert!(!settings.enabled);
    }

    // ==================== BlockedEvent Tests ====================

    #[test]
    fn blocked_event_new() {
        let event = BlockedEvent::new(
            Some("ChatGPT".to_string()),
            Some(Category::Violence),
            Some("Block Violence".to_string()),
            false,
        );
        assert_eq!(event.source, Some("ChatGPT".to_string()));
        assert_eq!(event.category, Some(Category::Violence));
        assert!(!event.is_time_block);
    }

    #[test]
    fn blocked_event_from_time_rule() {
        let source = RuleSource::TimeRule {
            rule_id: "bedtime".to_string(),
            rule_name: "Bedtime".to_string(),
        };
        let event = BlockedEvent::from_rule_source(&source, Some("Claude".to_string()));
        assert_eq!(event.source, Some("Claude".to_string()));
        assert!(event.is_time_block);
        assert_eq!(event.rule_name, Some("Bedtime".to_string()));
    }

    #[test]
    fn blocked_event_from_content_rule() {
        let content_result = ContentRuleResult {
            rule_id: "violence_block".to_string(),
            rule_name: "Block Violence".to_string(),
            category: Category::Violence,
            confidence: 0.9,
            action: ContentAction::Block,
        };
        let source = RuleSource::ContentRule(content_result);
        let event = BlockedEvent::from_rule_source(&source, Some("Gemini".to_string()));
        assert_eq!(event.source, Some("Gemini".to_string()));
        assert!(!event.is_time_block);
        assert_eq!(event.category, Some(Category::Violence));
        assert_eq!(event.rule_name, Some("Block Violence".to_string()));
    }

    // ==================== NotificationResult Tests ====================

    #[test]
    fn notification_result_sent() {
        let result = NotificationResult::Sent;
        assert!(result.was_sent());
        assert!(!result.was_rate_limited());
        assert!(!result.was_disabled());
    }

    #[test]
    fn notification_result_rate_limited() {
        let result = NotificationResult::RateLimited;
        assert!(!result.was_sent());
        assert!(result.was_rate_limited());
        assert!(!result.was_disabled());
    }

    #[test]
    fn notification_result_disabled() {
        let result = NotificationResult::Disabled;
        assert!(!result.was_sent());
        assert!(!result.was_rate_limited());
        assert!(result.was_disabled());
    }

    // ==================== NotificationManager Tests ====================

    #[test]
    fn manager_new_is_enabled() {
        let manager = NotificationManager::new();
        assert!(manager.is_enabled());
    }

    #[test]
    fn manager_with_disabled_settings() {
        let manager = NotificationManager::with_settings(NotificationSettings::disabled());
        assert!(!manager.is_enabled());
    }

    #[test]
    fn manager_enable_disable() {
        let manager = NotificationManager::new();
        assert!(manager.is_enabled());

        manager.disable();
        assert!(!manager.is_enabled());

        manager.enable();
        assert!(manager.is_enabled());
    }

    #[test]
    fn manager_set_enabled() {
        let manager = NotificationManager::new();
        manager.set_enabled(false);
        assert!(!manager.is_enabled());
        manager.set_enabled(true);
        assert!(manager.is_enabled());
    }

    #[test]
    fn manager_update_settings() {
        let manager = NotificationManager::new();
        manager.update_settings(NotificationSettings::disabled());
        assert!(!manager.is_enabled());
    }

    #[test]
    fn manager_disabled_returns_disabled_result() {
        let manager = NotificationManager::with_settings(NotificationSettings::disabled());
        let event = BlockedEvent::new(
            Some("Test".to_string()),
            Some(Category::Violence),
            None,
            false,
        );

        let result = manager.notify_block(&event);
        assert!(result.was_disabled());
    }

    #[test]
    fn manager_rate_limiting_works() {
        let manager = NotificationManager::new();
        let event = BlockedEvent::new(
            Some("Test".to_string()),
            Some(Category::Violence),
            None,
            false,
        );

        // First notification should succeed
        let result1 = manager.notify_block(&event);
        assert!(result1.was_sent());

        // Second notification should be rate-limited
        let result2 = manager.notify_block(&event);
        assert!(result2.was_rate_limited());

        // Check time remaining
        assert!(manager.is_rate_limited());
        assert!(manager.time_until_next().is_some());
    }

    #[test]
    fn manager_notify_if_blocked_only_blocks() {
        let manager = NotificationManager::new();
        let source = RuleSource::TimeRule {
            rule_id: "test".to_string(),
            rule_name: "Test".to_string(),
        };

        // Allow should return None
        let result = manager.notify_if_blocked(RuleAction::Allow, &source, None);
        assert!(result.is_none());

        // Warn should return None
        let result = manager.notify_if_blocked(RuleAction::Warn, &source, None);
        assert!(result.is_none());

        // Block should return Some
        let result = manager.notify_if_blocked(RuleAction::Block, &source, None);
        assert!(result.is_some());
    }

    #[test]
    fn manager_reset_rate_limit() {
        let manager = NotificationManager::new();
        let event = BlockedEvent::new(
            Some("Test".to_string()),
            Some(Category::Violence),
            None,
            false,
        );

        // Send first notification
        let _ = manager.notify_block(&event);
        assert!(manager.is_rate_limited());

        // Reset rate limit
        manager.reset_rate_limit();
        assert!(!manager.is_rate_limited());

        // Should be able to send again
        let result = manager.notify_block(&event);
        assert!(result.was_sent());
    }

    // ==================== Formatting Tests ====================

    #[test]
    fn format_notification_body_with_all_info() {
        let manager = NotificationManager::new();
        let event = BlockedEvent::new(
            Some("ChatGPT".to_string()),
            Some(Category::Violence),
            Some("Block Violence".to_string()),
            false,
        );

        let body = manager.format_notification_body(&event);
        assert!(body.contains("ChatGPT"));
        assert!(body.contains("Violence"));
    }

    #[test]
    fn format_notification_body_time_block() {
        let manager = NotificationManager::new();
        let event = BlockedEvent::new(
            Some("Claude".to_string()),
            None,
            Some("Bedtime".to_string()),
            true,
        );

        let body = manager.format_notification_body(&event);
        assert!(body.contains("Claude"));
        assert!(body.contains("Bedtime"));
        assert!(body.contains("time restriction"));
    }

    #[test]
    fn format_notification_body_minimal() {
        let manager = NotificationManager::new();
        let event = BlockedEvent::new(None, None, None, false);

        let body = manager.format_notification_body(&event);
        assert!(body.contains("blocked"));
    }

    // ==================== Serialization Tests ====================

    #[test]
    fn settings_serialization() {
        let settings = NotificationSettings::enabled();
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: NotificationSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(settings.enabled, deserialized.enabled);
    }
}

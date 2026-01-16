//! High-level database interface.

use std::path::PathBuf;

use aegis_core::classifier::Category;
use chrono::NaiveDate;
use directories::ProjectDirs;
use tracing::info;

use crate::error::{Result, StorageError};
use crate::models::{
    Action, Auth, Config, DailyStats, DisabledBundledSite, Event, NewEvent, NewProfile, NewRule,
    NewSite, Profile, Rule, Site,
};
use crate::pool::ConnectionPool;
use crate::repository::{
    create_preview, hash_prompt, AuthRepo, ConfigRepo, DisabledBundledRepo, EventsRepo,
    ProfileRepo, RulesRepo, SiteRepo, StatsRepo,
};

/// High-level database interface for Aegis.
#[derive(Clone)]
pub struct Database {
    pool: ConnectionPool,
}

impl Database {
    /// Create a new database in the default app data directory.
    pub fn new() -> Result<Self> {
        let path = Self::default_db_path()?;

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        info!("Opening database at: {:?}", path);
        let pool = ConnectionPool::new(&path)?;

        Ok(Self { pool })
    }

    /// Create a new database at a specific path.
    pub fn with_path(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        info!("Opening database at: {:?}", path);
        let pool = ConnectionPool::new(&path)?;

        Ok(Self { pool })
    }

    /// Create an in-memory database (for testing).
    pub fn in_memory() -> Result<Self> {
        let pool = ConnectionPool::in_memory()?;
        Ok(Self { pool })
    }

    /// Get the default database path.
    pub fn default_db_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "aegis", "aegis")
            .ok_or_else(|| StorageError::Config("Could not determine app data directory".into()))?;

        Ok(proj_dirs.data_dir().join("aegis.db"))
    }

    // === Events ===

    /// Log a new event from a prompt.
    pub fn log_event(
        &self,
        prompt: &str,
        category: Option<Category>,
        confidence: Option<f32>,
        action: Action,
        source: Option<String>,
    ) -> Result<i64> {
        let conn = self.pool.get()?;

        let event = NewEvent {
            prompt_hash: hash_prompt(prompt),
            preview: create_preview(prompt),
            category,
            confidence,
            action,
            source,
        };

        let id = EventsRepo::insert(&conn, event)?;

        // Update daily stats
        StatsRepo::increment(&conn, action, category)?;

        Ok(id)
    }

    /// Get an event by ID.
    pub fn get_event(&self, id: i64) -> Result<Option<Event>> {
        let conn = self.pool.get()?;
        EventsRepo::get_by_id(&conn, id)
    }

    /// Get recent events.
    pub fn get_recent_events(&self, limit: i64, offset: i64) -> Result<Vec<Event>> {
        let conn = self.pool.get()?;
        EventsRepo::get_recent(&conn, limit, offset)
    }

    /// Get events by action.
    pub fn get_events_by_action(
        &self,
        action: Action,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Event>> {
        let conn = self.pool.get()?;
        EventsRepo::get_by_action(&conn, action, limit, offset)
    }

    /// Count total events.
    pub fn count_events(&self) -> Result<i64> {
        let conn = self.pool.get()?;
        EventsRepo::count(&conn)
    }

    // === Rules ===

    /// Create a new rule.
    pub fn create_rule(&self, rule: NewRule) -> Result<i64> {
        let conn = self.pool.get()?;
        RulesRepo::insert(&conn, rule)
    }

    /// Get a rule by ID.
    pub fn get_rule(&self, id: i64) -> Result<Option<Rule>> {
        let conn = self.pool.get()?;
        RulesRepo::get_by_id(&conn, id)
    }

    /// Get a rule by name.
    pub fn get_rule_by_name(&self, name: &str) -> Result<Option<Rule>> {
        let conn = self.pool.get()?;
        RulesRepo::get_by_name(&conn, name)
    }

    /// Get all rules.
    pub fn get_all_rules(&self) -> Result<Vec<Rule>> {
        let conn = self.pool.get()?;
        RulesRepo::get_all(&conn)
    }

    /// Get all enabled rules.
    pub fn get_enabled_rules(&self) -> Result<Vec<Rule>> {
        let conn = self.pool.get()?;
        RulesRepo::get_enabled(&conn)
    }

    /// Update a rule.
    pub fn update_rule(&self, id: i64, rule: NewRule) -> Result<()> {
        let conn = self.pool.get()?;
        RulesRepo::update(&conn, id, rule)
    }

    /// Enable or disable a rule.
    pub fn set_rule_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        let conn = self.pool.get()?;
        RulesRepo::set_enabled(&conn, id, enabled)
    }

    /// Delete a rule.
    pub fn delete_rule(&self, id: i64) -> Result<()> {
        let conn = self.pool.get()?;
        RulesRepo::delete(&conn, id)
    }

    // === Stats ===

    /// Get stats for a specific date.
    pub fn get_stats(&self, date: NaiveDate) -> Result<Option<DailyStats>> {
        let conn = self.pool.get()?;
        StatsRepo::get_by_date(&conn, date)
    }

    /// Get stats for a date range.
    pub fn get_stats_range(&self, start: NaiveDate, end: NaiveDate) -> Result<Vec<DailyStats>> {
        let conn = self.pool.get()?;
        StatsRepo::get_range(&conn, start, end)
    }

    /// Get total aggregated stats.
    pub fn get_total_stats(&self) -> Result<DailyStats> {
        let conn = self.pool.get()?;
        StatsRepo::get_totals(&conn)
    }

    // === Config ===

    /// Get a configuration value.
    pub fn get_config(&self, key: &str) -> Result<Option<Config>> {
        let conn = self.pool.get()?;
        ConfigRepo::get(&conn, key)
    }

    /// Set a configuration value.
    pub fn set_config(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        let conn = self.pool.get()?;
        ConfigRepo::set(&conn, key, value)
    }

    /// Get a typed configuration value with default.
    pub fn get_config_or_default<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
        default: T,
    ) -> Result<T> {
        let conn = self.pool.get()?;
        ConfigRepo::get_or_default(&conn, key, default)
    }

    // === Auth ===

    /// Check if authentication is set up.
    pub fn is_auth_setup(&self) -> Result<bool> {
        let conn = self.pool.get()?;
        AuthRepo::is_setup(&conn)
    }

    /// Get auth info.
    pub fn get_auth(&self) -> Result<Option<Auth>> {
        let conn = self.pool.get()?;
        AuthRepo::get(&conn)
    }

    /// Set the password hash.
    pub fn set_password_hash(&self, hash: &str) -> Result<()> {
        let conn = self.pool.get()?;
        AuthRepo::set_password(&conn, hash)
    }

    /// Get the password hash.
    pub fn get_password_hash(&self) -> Result<String> {
        let conn = self.pool.get()?;
        AuthRepo::get_password_hash(&conn)
    }

    /// Update last login timestamp.
    pub fn update_last_login(&self) -> Result<()> {
        let conn = self.pool.get()?;
        AuthRepo::update_last_login(&conn)
    }

    // === Profiles ===

    /// Create a new profile.
    pub fn create_profile(&self, profile: NewProfile) -> Result<i64> {
        let conn = self.pool.get()?;
        ProfileRepo::insert(&conn, profile)
    }

    /// Get a profile by ID.
    pub fn get_profile(&self, id: i64) -> Result<Option<Profile>> {
        let conn = self.pool.get()?;
        ProfileRepo::get_by_id(&conn, id)
    }

    /// Get a profile by OS username.
    pub fn get_profile_by_os_username(&self, os_username: &str) -> Result<Option<Profile>> {
        let conn = self.pool.get()?;
        ProfileRepo::get_by_os_username(&conn, os_username)
    }

    /// Get all profiles.
    pub fn get_all_profiles(&self) -> Result<Vec<Profile>> {
        let conn = self.pool.get()?;
        ProfileRepo::get_all(&conn)
    }

    /// Get all enabled profiles.
    pub fn get_enabled_profiles(&self) -> Result<Vec<Profile>> {
        let conn = self.pool.get()?;
        ProfileRepo::get_enabled(&conn)
    }

    /// Update a profile.
    pub fn update_profile(&self, id: i64, profile: NewProfile) -> Result<()> {
        let conn = self.pool.get()?;
        ProfileRepo::update(&conn, id, profile)
    }

    /// Enable or disable a profile.
    pub fn set_profile_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        let conn = self.pool.get()?;
        ProfileRepo::set_enabled(&conn, id, enabled)
    }

    /// Delete a profile.
    pub fn delete_profile(&self, id: i64) -> Result<()> {
        let conn = self.pool.get()?;
        ProfileRepo::delete(&conn, id)
    }

    /// Count profiles.
    pub fn count_profiles(&self) -> Result<i64> {
        let conn = self.pool.get()?;
        ProfileRepo::count(&conn)
    }

    // === Protection State ===

    /// Config key for protection state.
    const PROTECTION_STATE_KEY: &'static str = "protection_state";

    /// Get the current protection state.
    ///
    /// Returns `None` if not set (defaults to Active).
    pub fn get_protection_state(&self) -> Result<Option<String>> {
        let conn = self.pool.get()?;
        match ConfigRepo::get(&conn, Self::PROTECTION_STATE_KEY)? {
            Some(config) => {
                if let Some(state) = config.value.as_str() {
                    Ok(Some(state.to_string()))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Set the protection state.
    pub fn set_protection_state(&self, state: &str) -> Result<()> {
        let conn = self.pool.get()?;
        ConfigRepo::set(&conn, Self::PROTECTION_STATE_KEY, &serde_json::json!(state))
    }

    // === Sites ===

    /// Create a new site.
    pub fn create_site(&self, site: NewSite) -> Result<i64> {
        let conn = self.pool.get()?;
        SiteRepo::insert(&conn, site)
    }

    /// Get a site by ID.
    pub fn get_site(&self, id: i64) -> Result<Option<Site>> {
        let conn = self.pool.get()?;
        SiteRepo::get_by_id(&conn, id)
    }

    /// Get a site by pattern.
    pub fn get_site_by_pattern(&self, pattern: &str) -> Result<Option<Site>> {
        let conn = self.pool.get()?;
        SiteRepo::get_by_pattern(&conn, pattern)
    }

    /// Get all sites.
    pub fn get_all_sites(&self) -> Result<Vec<Site>> {
        let conn = self.pool.get()?;
        SiteRepo::get_all(&conn)
    }

    /// Get all enabled sites.
    pub fn get_enabled_sites(&self) -> Result<Vec<Site>> {
        let conn = self.pool.get()?;
        SiteRepo::get_enabled(&conn)
    }

    /// Get sites by source.
    pub fn get_sites_by_source(&self, source: &str) -> Result<Vec<Site>> {
        let conn = self.pool.get()?;
        SiteRepo::get_by_source(&conn, source)
    }

    /// Update a site.
    pub fn update_site(&self, id: i64, site: NewSite) -> Result<()> {
        let conn = self.pool.get()?;
        SiteRepo::update(&conn, id, site)
    }

    /// Enable or disable a site.
    pub fn set_site_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        let conn = self.pool.get()?;
        SiteRepo::set_enabled(&conn, id, enabled)
    }

    /// Enable or disable a site by pattern.
    pub fn set_site_enabled_by_pattern(&self, pattern: &str, enabled: bool) -> Result<()> {
        let conn = self.pool.get()?;
        SiteRepo::set_enabled_by_pattern(&conn, pattern, enabled)
    }

    /// Delete a site.
    pub fn delete_site(&self, id: i64) -> Result<()> {
        let conn = self.pool.get()?;
        SiteRepo::delete(&conn, id)
    }

    /// Delete a site by pattern.
    pub fn delete_site_by_pattern(&self, pattern: &str) -> Result<()> {
        let conn = self.pool.get()?;
        SiteRepo::delete_by_pattern(&conn, pattern)
    }

    /// Count total sites.
    pub fn count_sites(&self) -> Result<i64> {
        let conn = self.pool.get()?;
        SiteRepo::count(&conn)
    }

    /// Upsert a site (insert or update by pattern).
    pub fn upsert_site(&self, site: NewSite) -> Result<i64> {
        let conn = self.pool.get()?;
        SiteRepo::upsert(&conn, site)
    }

    // === Disabled Bundled Sites ===

    /// Disable a bundled site.
    pub fn disable_bundled_site(&self, pattern: &str) -> Result<()> {
        let conn = self.pool.get()?;
        DisabledBundledRepo::add(&conn, pattern)
    }

    /// Re-enable a bundled site.
    pub fn enable_bundled_site(&self, pattern: &str) -> Result<()> {
        let conn = self.pool.get()?;
        DisabledBundledRepo::remove(&conn, pattern)
    }

    /// Check if a bundled site is disabled.
    pub fn is_bundled_site_disabled(&self, pattern: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        DisabledBundledRepo::is_disabled(&conn, pattern)
    }

    /// Get all disabled bundled sites.
    pub fn get_disabled_bundled_sites(&self) -> Result<Vec<DisabledBundledSite>> {
        let conn = self.pool.get()?;
        DisabledBundledRepo::get_all(&conn)
    }

    /// Get all disabled bundled site patterns.
    pub fn get_disabled_bundled_patterns(&self) -> Result<Vec<String>> {
        let conn = self.pool.get()?;
        DisabledBundledRepo::get_patterns(&conn)
    }

    /// Clear all disabled bundled sites.
    pub fn clear_disabled_bundled_sites(&self) -> Result<()> {
        let conn = self.pool.get()?;
        DisabledBundledRepo::clear(&conn)
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::in_memory().expect("Failed to create in-memory database")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_log_and_get_event() {
        let db = Database::in_memory().unwrap();

        let id = db
            .log_event(
                "test prompt",
                Some(Category::Violence),
                Some(0.95),
                Action::Blocked,
                Some("test".to_string()),
            )
            .unwrap();

        let event = db.get_event(id).unwrap().unwrap();
        assert_eq!(event.preview, "test prompt");
        assert_eq!(event.action, Action::Blocked);
    }

    #[test]
    fn test_rules_crud() {
        let db = Database::in_memory().unwrap();

        // Create
        let id = db
            .create_rule(NewRule {
                name: "test".to_string(),
                enabled: true,
                config: json!({"key": "value"}),
                priority: 0,
            })
            .unwrap();

        // Read
        let rule = db.get_rule(id).unwrap().unwrap();
        assert_eq!(rule.name, "test");

        // Update
        db.update_rule(
            id,
            NewRule {
                name: "updated".to_string(),
                enabled: false,
                config: json!({}),
                priority: 1,
            },
        )
        .unwrap();

        let rule = db.get_rule(id).unwrap().unwrap();
        assert_eq!(rule.name, "updated");

        // Delete
        db.delete_rule(id).unwrap();
        assert!(db.get_rule(id).unwrap().is_none());
    }

    #[test]
    fn test_stats_integration() {
        let db = Database::in_memory().unwrap();

        // Log some events
        db.log_event(
            "test 1",
            Some(Category::Violence),
            Some(0.9),
            Action::Blocked,
            None,
        )
        .unwrap();
        db.log_event("test 2", None, None, Action::Allowed, None)
            .unwrap();
        db.log_event(
            "test 3",
            Some(Category::Jailbreak),
            Some(0.8),
            Action::Blocked,
            None,
        )
        .unwrap();

        // Check totals
        let stats = db.get_total_stats().unwrap();
        assert_eq!(stats.total_prompts, 3);
        assert_eq!(stats.blocked_count, 2);
        assert_eq!(stats.allowed_count, 1);
    }

    #[test]
    fn test_config() {
        let db = Database::in_memory().unwrap();

        db.set_config("test_key", &json!({"nested": true})).unwrap();
        let config = db.get_config("test_key").unwrap().unwrap();
        assert_eq!(config.value["nested"], true);
    }

    #[test]
    fn test_auth() {
        let db = Database::in_memory().unwrap();

        assert!(!db.is_auth_setup().unwrap());

        db.set_password_hash("hash123").unwrap();
        assert!(db.is_auth_setup().unwrap());

        let hash = db.get_password_hash().unwrap();
        assert_eq!(hash, "hash123");
    }

    #[test]
    fn test_protection_state() {
        let db = Database::in_memory().unwrap();

        // Default is None (Active)
        assert!(db.get_protection_state().unwrap().is_none());

        // Set to paused
        db.set_protection_state("paused").unwrap();
        assert_eq!(
            db.get_protection_state().unwrap(),
            Some("paused".to_string())
        );

        // Set to disabled
        db.set_protection_state("disabled").unwrap();
        assert_eq!(
            db.get_protection_state().unwrap(),
            Some("disabled".to_string())
        );

        // Set back to active
        db.set_protection_state("active").unwrap();
        assert_eq!(
            db.get_protection_state().unwrap(),
            Some("active".to_string())
        );
    }
}

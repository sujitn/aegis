//! High-level database interface.

use std::path::PathBuf;

use aegis_core::classifier::Category;
use chrono::NaiveDate;
use directories::ProjectDirs;
use tracing::info;

use crate::error::{Result, StorageError};
use crate::models::{Action, Auth, Config, DailyStats, Event, NewEvent, NewRule, Rule};
use crate::pool::ConnectionPool;
use crate::repository::{
    create_preview, hash_prompt, AuthRepo, ConfigRepo, EventsRepo, RulesRepo, StatsRepo,
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
}

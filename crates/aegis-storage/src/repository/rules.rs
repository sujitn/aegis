//! Rules repository.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

use crate::error::{Result, StorageError};
use crate::models::{NewRule, Rule};

/// Repository for rule operations.
pub struct RulesRepo;

impl RulesRepo {
    /// Insert a new rule.
    pub fn insert(conn: &Connection, rule: NewRule) -> Result<i64> {
        let config_json = serde_json::to_string(&rule.config)?;

        conn.execute(
            "INSERT INTO rules (name, enabled, config, priority)
             VALUES (?1, ?2, ?3, ?4)",
            params![rule.name, rule.enabled as i32, config_json, rule.priority],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get a rule by ID.
    pub fn get_by_id(conn: &Connection, id: i64) -> Result<Option<Rule>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, enabled, config, priority, created_at, updated_at
             FROM rules WHERE id = ?1",
        )?;

        let rule = stmt
            .query_row([id], |row| {
                let config_str: String = row.get(3)?;
                Ok(Rule {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    enabled: row.get::<_, i32>(2)? != 0,
                    config: serde_json::from_str(&config_str).unwrap_or(serde_json::Value::Null),
                    priority: row.get(4)?,
                    created_at: parse_datetime(&row.get::<_, String>(5)?),
                    updated_at: parse_datetime(&row.get::<_, String>(6)?),
                })
            })
            .ok();

        Ok(rule)
    }

    /// Get a rule by name.
    pub fn get_by_name(conn: &Connection, name: &str) -> Result<Option<Rule>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, enabled, config, priority, created_at, updated_at
             FROM rules WHERE name = ?1",
        )?;

        let rule = stmt
            .query_row([name], |row| {
                let config_str: String = row.get(3)?;
                Ok(Rule {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    enabled: row.get::<_, i32>(2)? != 0,
                    config: serde_json::from_str(&config_str).unwrap_or(serde_json::Value::Null),
                    priority: row.get(4)?,
                    created_at: parse_datetime(&row.get::<_, String>(5)?),
                    updated_at: parse_datetime(&row.get::<_, String>(6)?),
                })
            })
            .ok();

        Ok(rule)
    }

    /// Get all rules, ordered by priority.
    pub fn get_all(conn: &Connection) -> Result<Vec<Rule>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, enabled, config, priority, created_at, updated_at
             FROM rules ORDER BY priority ASC",
        )?;

        let rules = stmt
            .query_map([], |row| {
                let config_str: String = row.get(3)?;
                Ok(Rule {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    enabled: row.get::<_, i32>(2)? != 0,
                    config: serde_json::from_str(&config_str).unwrap_or(serde_json::Value::Null),
                    priority: row.get(4)?,
                    created_at: parse_datetime(&row.get::<_, String>(5)?),
                    updated_at: parse_datetime(&row.get::<_, String>(6)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rules)
    }

    /// Get all enabled rules, ordered by priority.
    pub fn get_enabled(conn: &Connection) -> Result<Vec<Rule>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, enabled, config, priority, created_at, updated_at
             FROM rules WHERE enabled = 1 ORDER BY priority ASC",
        )?;

        let rules = stmt
            .query_map([], |row| {
                let config_str: String = row.get(3)?;
                Ok(Rule {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    enabled: row.get::<_, i32>(2)? != 0,
                    config: serde_json::from_str(&config_str).unwrap_or(serde_json::Value::Null),
                    priority: row.get(4)?,
                    created_at: parse_datetime(&row.get::<_, String>(5)?),
                    updated_at: parse_datetime(&row.get::<_, String>(6)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rules)
    }

    /// Update a rule.
    pub fn update(conn: &Connection, id: i64, rule: NewRule) -> Result<()> {
        let config_json = serde_json::to_string(&rule.config)?;

        let updated = conn.execute(
            "UPDATE rules SET name = ?1, enabled = ?2, config = ?3, priority = ?4,
             updated_at = datetime('now') WHERE id = ?5",
            params![
                rule.name,
                rule.enabled as i32,
                config_json,
                rule.priority,
                id
            ],
        )?;

        if updated == 0 {
            return Err(StorageError::NotFound(format!("Rule with id {}", id)));
        }

        Ok(())
    }

    /// Enable or disable a rule.
    pub fn set_enabled(conn: &Connection, id: i64, enabled: bool) -> Result<()> {
        let updated = conn.execute(
            "UPDATE rules SET enabled = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![enabled as i32, id],
        )?;

        if updated == 0 {
            return Err(StorageError::NotFound(format!("Rule with id {}", id)));
        }

        Ok(())
    }

    /// Delete a rule.
    pub fn delete(conn: &Connection, id: i64) -> Result<()> {
        let deleted = conn.execute("DELETE FROM rules WHERE id = ?1", [id])?;

        if deleted == 0 {
            return Err(StorageError::NotFound(format!("Rule with id {}", id)));
        }

        Ok(())
    }

    /// Count total rules.
    pub fn count(conn: &Connection) -> Result<i64> {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM rules", [], |row| row.get(0))?;
        Ok(count)
    }
}

/// Parse a datetime from SQLite format.
fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").map(|dt| dt.and_utc())
        })
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::run_migrations;
    use serde_json::json;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_insert_and_get_rule() {
        let conn = setup_db();

        let rule = NewRule {
            name: "test_rule".to_string(),
            enabled: true,
            config: json!({"key": "value"}),
            priority: 10,
        };

        let id = RulesRepo::insert(&conn, rule).unwrap();
        let retrieved = RulesRepo::get_by_id(&conn, id).unwrap().unwrap();

        assert_eq!(retrieved.name, "test_rule");
        assert!(retrieved.enabled);
        assert_eq!(retrieved.priority, 10);
        assert_eq!(retrieved.config["key"], "value");
    }

    #[test]
    fn test_get_by_name() {
        let conn = setup_db();

        let rule = NewRule {
            name: "named_rule".to_string(),
            enabled: true,
            config: json!({}),
            priority: 0,
        };

        RulesRepo::insert(&conn, rule).unwrap();
        let retrieved = RulesRepo::get_by_name(&conn, "named_rule")
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.name, "named_rule");
    }

    #[test]
    fn test_get_enabled_rules() {
        let conn = setup_db();

        // Insert enabled rule
        RulesRepo::insert(
            &conn,
            NewRule {
                name: "enabled".to_string(),
                enabled: true,
                config: json!({}),
                priority: 0,
            },
        )
        .unwrap();

        // Insert disabled rule
        RulesRepo::insert(
            &conn,
            NewRule {
                name: "disabled".to_string(),
                enabled: false,
                config: json!({}),
                priority: 0,
            },
        )
        .unwrap();

        let enabled = RulesRepo::get_enabled(&conn).unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "enabled");
    }

    #[test]
    fn test_update_rule() {
        let conn = setup_db();

        let rule = NewRule {
            name: "original".to_string(),
            enabled: true,
            config: json!({}),
            priority: 0,
        };

        let id = RulesRepo::insert(&conn, rule).unwrap();

        RulesRepo::update(
            &conn,
            id,
            NewRule {
                name: "updated".to_string(),
                enabled: false,
                config: json!({"new": true}),
                priority: 5,
            },
        )
        .unwrap();

        let updated = RulesRepo::get_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(updated.name, "updated");
        assert!(!updated.enabled);
        assert_eq!(updated.priority, 5);
    }

    #[test]
    fn test_delete_rule() {
        let conn = setup_db();

        let rule = NewRule {
            name: "to_delete".to_string(),
            enabled: true,
            config: json!({}),
            priority: 0,
        };

        let id = RulesRepo::insert(&conn, rule).unwrap();
        assert!(RulesRepo::get_by_id(&conn, id).unwrap().is_some());

        RulesRepo::delete(&conn, id).unwrap();
        assert!(RulesRepo::get_by_id(&conn, id).unwrap().is_none());
    }

    #[test]
    fn test_rules_ordered_by_priority() {
        let conn = setup_db();

        RulesRepo::insert(
            &conn,
            NewRule {
                name: "low".to_string(),
                enabled: true,
                config: json!({}),
                priority: 100,
            },
        )
        .unwrap();

        RulesRepo::insert(
            &conn,
            NewRule {
                name: "high".to_string(),
                enabled: true,
                config: json!({}),
                priority: 1,
            },
        )
        .unwrap();

        let rules = RulesRepo::get_all(&conn).unwrap();
        assert_eq!(rules[0].name, "high");
        assert_eq!(rules[1].name, "low");
    }
}

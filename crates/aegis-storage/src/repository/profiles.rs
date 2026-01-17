//! Profile repository.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

use crate::error::{Result, StorageError};
#[cfg(test)]
use crate::models::ProfileSentimentConfig;
use crate::models::{NewProfile, Profile};

/// Repository for profile operations.
pub struct ProfileRepo;

impl ProfileRepo {
    /// Insert a new profile.
    pub fn insert(conn: &Connection, profile: NewProfile) -> Result<i64> {
        let time_rules_json = serde_json::to_string(&profile.time_rules)?;
        let content_rules_json = serde_json::to_string(&profile.content_rules)?;
        let sentiment_config_json = serde_json::to_string(&profile.sentiment_config)?;

        conn.execute(
            "INSERT INTO profiles (name, os_username, time_rules, content_rules, enabled, sentiment_config)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                profile.name,
                profile.os_username,
                time_rules_json,
                content_rules_json,
                profile.enabled as i32,
                sentiment_config_json
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get a profile by ID.
    pub fn get_by_id(conn: &Connection, id: i64) -> Result<Option<Profile>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, os_username, time_rules, content_rules, enabled, sentiment_config, created_at, updated_at
             FROM profiles WHERE id = ?1",
        )?;

        let profile = stmt
            .query_row([id], |row| {
                let time_rules_str: String = row.get(3)?;
                let content_rules_str: String = row.get(4)?;
                let sentiment_config_str: String = row.get(6)?;
                Ok(Profile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    os_username: row.get(2)?,
                    time_rules: serde_json::from_str(&time_rules_str)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
                    content_rules: serde_json::from_str(&content_rules_str)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
                    enabled: row.get::<_, i32>(5)? != 0,
                    sentiment_config: serde_json::from_str(&sentiment_config_str)
                        .unwrap_or_default(),
                    created_at: parse_datetime(&row.get::<_, String>(7)?),
                    updated_at: parse_datetime(&row.get::<_, String>(8)?),
                })
            })
            .ok();

        Ok(profile)
    }

    /// Get a profile by OS username.
    pub fn get_by_os_username(conn: &Connection, os_username: &str) -> Result<Option<Profile>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, os_username, time_rules, content_rules, enabled, sentiment_config, created_at, updated_at
             FROM profiles WHERE os_username = ?1 AND enabled = 1",
        )?;

        let profile = stmt
            .query_row([os_username], |row| {
                let time_rules_str: String = row.get(3)?;
                let content_rules_str: String = row.get(4)?;
                let sentiment_config_str: String = row.get(6)?;
                Ok(Profile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    os_username: row.get(2)?,
                    time_rules: serde_json::from_str(&time_rules_str)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
                    content_rules: serde_json::from_str(&content_rules_str)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
                    enabled: row.get::<_, i32>(5)? != 0,
                    sentiment_config: serde_json::from_str(&sentiment_config_str)
                        .unwrap_or_default(),
                    created_at: parse_datetime(&row.get::<_, String>(7)?),
                    updated_at: parse_datetime(&row.get::<_, String>(8)?),
                })
            })
            .ok();

        Ok(profile)
    }

    /// Get all profiles.
    pub fn get_all(conn: &Connection) -> Result<Vec<Profile>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, os_username, time_rules, content_rules, enabled, sentiment_config, created_at, updated_at
             FROM profiles ORDER BY name ASC",
        )?;

        let profiles = stmt
            .query_map([], |row| {
                let time_rules_str: String = row.get(3)?;
                let content_rules_str: String = row.get(4)?;
                let sentiment_config_str: String = row.get(6)?;
                Ok(Profile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    os_username: row.get(2)?,
                    time_rules: serde_json::from_str(&time_rules_str)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
                    content_rules: serde_json::from_str(&content_rules_str)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
                    enabled: row.get::<_, i32>(5)? != 0,
                    sentiment_config: serde_json::from_str(&sentiment_config_str)
                        .unwrap_or_default(),
                    created_at: parse_datetime(&row.get::<_, String>(7)?),
                    updated_at: parse_datetime(&row.get::<_, String>(8)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(profiles)
    }

    /// Get all enabled profiles.
    pub fn get_enabled(conn: &Connection) -> Result<Vec<Profile>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, os_username, time_rules, content_rules, enabled, sentiment_config, created_at, updated_at
             FROM profiles WHERE enabled = 1 ORDER BY name ASC",
        )?;

        let profiles = stmt
            .query_map([], |row| {
                let time_rules_str: String = row.get(3)?;
                let content_rules_str: String = row.get(4)?;
                let sentiment_config_str: String = row.get(6)?;
                Ok(Profile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    os_username: row.get(2)?,
                    time_rules: serde_json::from_str(&time_rules_str)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
                    content_rules: serde_json::from_str(&content_rules_str)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
                    enabled: row.get::<_, i32>(5)? != 0,
                    sentiment_config: serde_json::from_str(&sentiment_config_str)
                        .unwrap_or_default(),
                    created_at: parse_datetime(&row.get::<_, String>(7)?),
                    updated_at: parse_datetime(&row.get::<_, String>(8)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(profiles)
    }

    /// Update a profile.
    pub fn update(conn: &Connection, id: i64, profile: NewProfile) -> Result<()> {
        let time_rules_json = serde_json::to_string(&profile.time_rules)?;
        let content_rules_json = serde_json::to_string(&profile.content_rules)?;
        let sentiment_config_json = serde_json::to_string(&profile.sentiment_config)?;

        let updated = conn.execute(
            "UPDATE profiles SET name = ?1, os_username = ?2, time_rules = ?3, content_rules = ?4,
             enabled = ?5, sentiment_config = ?6, updated_at = datetime('now') WHERE id = ?7",
            params![
                profile.name,
                profile.os_username,
                time_rules_json,
                content_rules_json,
                profile.enabled as i32,
                sentiment_config_json,
                id
            ],
        )?;

        if updated == 0 {
            return Err(StorageError::NotFound(format!("Profile with id {}", id)));
        }

        Ok(())
    }

    /// Enable or disable a profile.
    pub fn set_enabled(conn: &Connection, id: i64, enabled: bool) -> Result<()> {
        let updated = conn.execute(
            "UPDATE profiles SET enabled = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![enabled as i32, id],
        )?;

        if updated == 0 {
            return Err(StorageError::NotFound(format!("Profile with id {}", id)));
        }

        Ok(())
    }

    /// Delete a profile.
    pub fn delete(conn: &Connection, id: i64) -> Result<()> {
        let deleted = conn.execute("DELETE FROM profiles WHERE id = ?1", [id])?;

        if deleted == 0 {
            return Err(StorageError::NotFound(format!("Profile with id {}", id)));
        }

        Ok(())
    }

    /// Count total profiles.
    pub fn count(conn: &Connection) -> Result<i64> {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM profiles", [], |row| row.get(0))?;
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
    fn test_insert_and_get_profile() {
        let conn = setup_db();

        let profile = NewProfile {
            name: "Test Child".to_string(),
            os_username: Some("testchild".to_string()),
            time_rules: json!({"rules": []}),
            content_rules: json!({"rules": []}),
            enabled: true,
            sentiment_config: ProfileSentimentConfig::default(),
        };

        let id = ProfileRepo::insert(&conn, profile).unwrap();
        let retrieved = ProfileRepo::get_by_id(&conn, id).unwrap().unwrap();

        assert_eq!(retrieved.name, "Test Child");
        assert_eq!(retrieved.os_username, Some("testchild".to_string()));
        assert!(retrieved.enabled);
    }

    #[test]
    fn test_get_by_os_username() {
        let conn = setup_db();

        let profile = NewProfile {
            name: "Child".to_string(),
            os_username: Some("childuser".to_string()),
            time_rules: json!({}),
            content_rules: json!({}),
            enabled: true,
            sentiment_config: ProfileSentimentConfig::default(),
        };

        ProfileRepo::insert(&conn, profile).unwrap();
        let retrieved = ProfileRepo::get_by_os_username(&conn, "childuser")
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.name, "Child");
    }

    #[test]
    fn test_get_by_os_username_disabled() {
        let conn = setup_db();

        let profile = NewProfile {
            name: "Child".to_string(),
            os_username: Some("disabledchild".to_string()),
            time_rules: json!({}),
            content_rules: json!({}),
            enabled: false,
            sentiment_config: ProfileSentimentConfig::default(),
        };

        ProfileRepo::insert(&conn, profile).unwrap();

        // Should not find disabled profile
        let retrieved = ProfileRepo::get_by_os_username(&conn, "disabledchild").unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_get_all_profiles() {
        let conn = setup_db();

        ProfileRepo::insert(
            &conn,
            NewProfile {
                name: "Alice".to_string(),
                os_username: Some("alice".to_string()),
                time_rules: json!({}),
                content_rules: json!({}),
                enabled: true,
                sentiment_config: ProfileSentimentConfig::default(),
            },
        )
        .unwrap();

        ProfileRepo::insert(
            &conn,
            NewProfile {
                name: "Bob".to_string(),
                os_username: Some("bob".to_string()),
                time_rules: json!({}),
                content_rules: json!({}),
                enabled: true,
                sentiment_config: ProfileSentimentConfig::default(),
            },
        )
        .unwrap();

        let all = ProfileRepo::get_all(&conn).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_get_enabled_profiles() {
        let conn = setup_db();

        ProfileRepo::insert(
            &conn,
            NewProfile {
                name: "Enabled".to_string(),
                os_username: Some("enabled".to_string()),
                time_rules: json!({}),
                content_rules: json!({}),
                enabled: true,
                sentiment_config: ProfileSentimentConfig::default(),
            },
        )
        .unwrap();

        ProfileRepo::insert(
            &conn,
            NewProfile {
                name: "Disabled".to_string(),
                os_username: Some("disabled".to_string()),
                time_rules: json!({}),
                content_rules: json!({}),
                enabled: false,
                sentiment_config: ProfileSentimentConfig::default(),
            },
        )
        .unwrap();

        let enabled = ProfileRepo::get_enabled(&conn).unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "Enabled");
    }

    #[test]
    fn test_update_profile() {
        let conn = setup_db();

        let profile = NewProfile {
            name: "Original".to_string(),
            os_username: Some("user".to_string()),
            time_rules: json!({}),
            content_rules: json!({}),
            enabled: true,
            sentiment_config: ProfileSentimentConfig::default(),
        };

        let id = ProfileRepo::insert(&conn, profile).unwrap();

        ProfileRepo::update(
            &conn,
            id,
            NewProfile {
                name: "Updated".to_string(),
                os_username: Some("newuser".to_string()),
                time_rules: json!({"updated": true}),
                content_rules: json!({}),
                enabled: false,
                sentiment_config: ProfileSentimentConfig::default(),
            },
        )
        .unwrap();

        let updated = ProfileRepo::get_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.os_username, Some("newuser".to_string()));
        assert!(!updated.enabled);
    }

    #[test]
    fn test_delete_profile() {
        let conn = setup_db();

        let profile = NewProfile {
            name: "ToDelete".to_string(),
            os_username: None,
            time_rules: json!({}),
            content_rules: json!({}),
            enabled: true,
            sentiment_config: ProfileSentimentConfig::default(),
        };

        let id = ProfileRepo::insert(&conn, profile).unwrap();
        assert!(ProfileRepo::get_by_id(&conn, id).unwrap().is_some());

        ProfileRepo::delete(&conn, id).unwrap();
        assert!(ProfileRepo::get_by_id(&conn, id).unwrap().is_none());
    }

    #[test]
    fn test_set_enabled() {
        let conn = setup_db();

        let profile = NewProfile {
            name: "Test".to_string(),
            os_username: None,
            time_rules: json!({}),
            content_rules: json!({}),
            enabled: true,
            sentiment_config: ProfileSentimentConfig::default(),
        };

        let id = ProfileRepo::insert(&conn, profile).unwrap();
        assert!(ProfileRepo::get_by_id(&conn, id).unwrap().unwrap().enabled);

        ProfileRepo::set_enabled(&conn, id, false).unwrap();
        assert!(!ProfileRepo::get_by_id(&conn, id).unwrap().unwrap().enabled);
    }

    #[test]
    fn test_count_profiles() {
        let conn = setup_db();
        assert_eq!(ProfileRepo::count(&conn).unwrap(), 0);

        ProfileRepo::insert(
            &conn,
            NewProfile {
                name: "One".to_string(),
                os_username: None,
                time_rules: json!({}),
                content_rules: json!({}),
                enabled: true,
                sentiment_config: ProfileSentimentConfig::default(),
            },
        )
        .unwrap();

        assert_eq!(ProfileRepo::count(&conn).unwrap(), 1);
    }

    #[test]
    fn test_profile_with_no_os_username() {
        let conn = setup_db();

        let profile = NewProfile {
            name: "Manual Only".to_string(),
            os_username: None,
            time_rules: json!({}),
            content_rules: json!({}),
            enabled: true,
            sentiment_config: ProfileSentimentConfig::default(),
        };

        let id = ProfileRepo::insert(&conn, profile).unwrap();
        let retrieved = ProfileRepo::get_by_id(&conn, id).unwrap().unwrap();

        assert_eq!(retrieved.name, "Manual Only");
        assert!(retrieved.os_username.is_none());
    }
}

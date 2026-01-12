//! Configuration repository.

use rusqlite::{params, Connection};

use crate::error::Result;
use crate::models::Config;

/// Repository for configuration operations.
pub struct ConfigRepo;

impl ConfigRepo {
    /// Get a configuration value.
    pub fn get(conn: &Connection, key: &str) -> Result<Option<Config>> {
        let mut stmt = conn.prepare("SELECT key, value FROM config WHERE key = ?1")?;

        let config = stmt
            .query_row([key], |row| {
                let value_str: String = row.get(1)?;
                Ok(Config {
                    key: row.get(0)?,
                    value: serde_json::from_str(&value_str).unwrap_or(serde_json::Value::Null),
                })
            })
            .ok();

        Ok(config)
    }

    /// Set a configuration value (insert or update).
    pub fn set(conn: &Connection, key: &str, value: &serde_json::Value) -> Result<()> {
        let value_json = serde_json::to_string(value)?;

        conn.execute(
            "INSERT INTO config (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value_json],
        )?;

        Ok(())
    }

    /// Delete a configuration value.
    pub fn delete(conn: &Connection, key: &str) -> Result<bool> {
        let deleted = conn.execute("DELETE FROM config WHERE key = ?1", [key])?;
        Ok(deleted > 0)
    }

    /// Get all configuration values.
    pub fn get_all(conn: &Connection) -> Result<Vec<Config>> {
        let mut stmt = conn.prepare("SELECT key, value FROM config ORDER BY key")?;

        let configs = stmt
            .query_map([], |row| {
                let value_str: String = row.get(1)?;
                Ok(Config {
                    key: row.get(0)?,
                    value: serde_json::from_str(&value_str).unwrap_or(serde_json::Value::Null),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(configs)
    }

    /// Get a typed configuration value with a default.
    pub fn get_or_default<T: serde::de::DeserializeOwned>(
        conn: &Connection,
        key: &str,
        default: T,
    ) -> Result<T> {
        match Self::get(conn, key)? {
            Some(config) => Ok(serde_json::from_value(config.value).unwrap_or(default)),
            None => Ok(default),
        }
    }
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
    fn test_set_and_get() {
        let conn = setup_db();

        ConfigRepo::set(&conn, "test_key", &json!("test_value")).unwrap();
        let config = ConfigRepo::get(&conn, "test_key").unwrap().unwrap();

        assert_eq!(config.key, "test_key");
        assert_eq!(config.value, json!("test_value"));
    }

    #[test]
    fn test_update_existing() {
        let conn = setup_db();

        ConfigRepo::set(&conn, "key", &json!("original")).unwrap();
        ConfigRepo::set(&conn, "key", &json!("updated")).unwrap();

        let config = ConfigRepo::get(&conn, "key").unwrap().unwrap();
        assert_eq!(config.value, json!("updated"));
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = setup_db();
        let config = ConfigRepo::get(&conn, "nonexistent").unwrap();
        assert!(config.is_none());
    }

    #[test]
    fn test_delete() {
        let conn = setup_db();

        ConfigRepo::set(&conn, "to_delete", &json!("value")).unwrap();
        assert!(ConfigRepo::get(&conn, "to_delete").unwrap().is_some());

        let deleted = ConfigRepo::delete(&conn, "to_delete").unwrap();
        assert!(deleted);
        assert!(ConfigRepo::get(&conn, "to_delete").unwrap().is_none());
    }

    #[test]
    fn test_get_all() {
        let conn = setup_db();

        ConfigRepo::set(&conn, "a", &json!(1)).unwrap();
        ConfigRepo::set(&conn, "b", &json!(2)).unwrap();
        ConfigRepo::set(&conn, "c", &json!(3)).unwrap();

        let configs = ConfigRepo::get_all(&conn).unwrap();
        assert_eq!(configs.len(), 3);
    }

    #[test]
    fn test_get_or_default() {
        let conn = setup_db();

        // Non-existent key returns default
        let value: i32 = ConfigRepo::get_or_default(&conn, "missing", 42).unwrap();
        assert_eq!(value, 42);

        // Existing key returns stored value
        ConfigRepo::set(&conn, "existing", &json!(100)).unwrap();
        let value: i32 = ConfigRepo::get_or_default(&conn, "existing", 42).unwrap();
        assert_eq!(value, 100);
    }

    #[test]
    fn test_complex_values() {
        let conn = setup_db();

        let complex = json!({
            "nested": {
                "array": [1, 2, 3],
                "bool": true
            }
        });

        ConfigRepo::set(&conn, "complex", &complex).unwrap();
        let config = ConfigRepo::get(&conn, "complex").unwrap().unwrap();

        assert_eq!(config.value["nested"]["array"][0], 1);
        assert_eq!(config.value["nested"]["bool"], true);
    }
}

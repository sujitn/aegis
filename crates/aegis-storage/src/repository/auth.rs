//! Authentication repository.

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::error::{Result, StorageError};
use crate::models::Auth;

/// Repository for authentication operations.
pub struct AuthRepo;

impl AuthRepo {
    /// Check if a password is set (user exists).
    pub fn is_setup(conn: &Connection) -> Result<bool> {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM auth WHERE id = 1", [], |row| {
            row.get(0)
        })?;
        Ok(count > 0)
    }

    /// Get the auth record.
    pub fn get(conn: &Connection) -> Result<Option<Auth>> {
        let mut stmt = conn
            .prepare("SELECT id, password_hash, created_at, last_login FROM auth WHERE id = 1")?;

        let auth = stmt
            .query_row([], |row| {
                Ok(Auth {
                    id: row.get(0)?,
                    password_hash: row.get(1)?,
                    created_at: parse_datetime(&row.get::<_, String>(2)?),
                    last_login: row.get::<_, Option<String>>(3)?.map(|s| parse_datetime(&s)),
                })
            })
            .ok();

        Ok(auth)
    }

    /// Set the password hash (for initial setup or password change).
    pub fn set_password(conn: &Connection, password_hash: &str) -> Result<()> {
        conn.execute(
            "INSERT INTO auth (id, password_hash) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET password_hash = ?1",
            [password_hash],
        )?;

        Ok(())
    }

    /// Update the last login timestamp.
    pub fn update_last_login(conn: &Connection) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        conn.execute("UPDATE auth SET last_login = ?1 WHERE id = 1", [&now])?;
        Ok(())
    }

    /// Get the password hash for verification.
    pub fn get_password_hash(conn: &Connection) -> Result<String> {
        let hash: String = conn
            .query_row("SELECT password_hash FROM auth WHERE id = 1", [], |row| {
                row.get(0)
            })
            .map_err(|_| StorageError::NotFound("Auth not setup".to_string()))?;

        Ok(hash)
    }

    /// Delete auth record (for testing or reset).
    pub fn delete(conn: &Connection) -> Result<()> {
        conn.execute("DELETE FROM auth WHERE id = 1", [])?;
        Ok(())
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

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_is_setup() {
        let conn = setup_db();

        // Initially not setup
        assert!(!AuthRepo::is_setup(&conn).unwrap());

        // After setting password, is setup
        AuthRepo::set_password(&conn, "hash123").unwrap();
        assert!(AuthRepo::is_setup(&conn).unwrap());
    }

    #[test]
    fn test_set_and_get_password() {
        let conn = setup_db();

        AuthRepo::set_password(&conn, "test_hash").unwrap();
        let hash = AuthRepo::get_password_hash(&conn).unwrap();
        assert_eq!(hash, "test_hash");
    }

    #[test]
    fn test_update_password() {
        let conn = setup_db();

        AuthRepo::set_password(&conn, "original").unwrap();
        AuthRepo::set_password(&conn, "updated").unwrap();

        let hash = AuthRepo::get_password_hash(&conn).unwrap();
        assert_eq!(hash, "updated");
    }

    #[test]
    fn test_get_auth() {
        let conn = setup_db();

        AuthRepo::set_password(&conn, "hash").unwrap();
        let auth = AuthRepo::get(&conn).unwrap().unwrap();

        assert_eq!(auth.id, 1);
        assert_eq!(auth.password_hash, "hash");
        assert!(auth.last_login.is_none());
    }

    #[test]
    fn test_update_last_login() {
        let conn = setup_db();

        AuthRepo::set_password(&conn, "hash").unwrap();
        AuthRepo::update_last_login(&conn).unwrap();

        let auth = AuthRepo::get(&conn).unwrap().unwrap();
        assert!(auth.last_login.is_some());
    }

    #[test]
    fn test_delete() {
        let conn = setup_db();

        AuthRepo::set_password(&conn, "hash").unwrap();
        assert!(AuthRepo::is_setup(&conn).unwrap());

        AuthRepo::delete(&conn).unwrap();
        assert!(!AuthRepo::is_setup(&conn).unwrap());
    }
}

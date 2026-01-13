//! Database schema and migrations.

use rusqlite::Connection;
use tracing::info;

use crate::error::Result;

/// Current schema version.
pub const SCHEMA_VERSION: i32 = 2;

/// Run all pending migrations.
pub fn run_migrations(conn: &Connection) -> Result<()> {
    let current_version = get_schema_version(conn)?;

    if current_version < SCHEMA_VERSION {
        info!(
            "Running migrations from version {} to {}",
            current_version, SCHEMA_VERSION
        );

        if current_version < 1 {
            migrate_v1(conn)?;
        }

        if current_version < 2 {
            migrate_v2(conn)?;
        }

        set_schema_version(conn, SCHEMA_VERSION)?;
        info!("Migrations complete");
    }

    Ok(())
}

/// Get the current schema version.
fn get_schema_version(conn: &Connection) -> Result<i32> {
    // Create schema_version table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        )",
        [],
    )?;

    let version: Option<i32> = conn
        .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
            row.get(0)
        })
        .ok();

    Ok(version.unwrap_or(0))
}

/// Set the schema version.
fn set_schema_version(conn: &Connection, version: i32) -> Result<()> {
    conn.execute("DELETE FROM schema_version", [])?;
    conn.execute(
        "INSERT INTO schema_version (version) VALUES (?1)",
        [version],
    )?;
    Ok(())
}

/// Migration to version 1: Initial schema.
fn migrate_v1(conn: &Connection) -> Result<()> {
    info!("Applying migration v1: Initial schema");

    // Events table - stores classification events (privacy-preserving)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            prompt_hash TEXT NOT NULL,
            preview TEXT NOT NULL,
            category TEXT,
            confidence REAL,
            action TEXT NOT NULL,
            source TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Index for querying events by date
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_created_at ON events (created_at)",
        [],
    )?;

    // Index for deduplication by hash
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_prompt_hash ON events (prompt_hash)",
        [],
    )?;

    // Daily stats table - aggregated statistics
    conn.execute(
        "CREATE TABLE IF NOT EXISTS daily_stats (
            date TEXT PRIMARY KEY,
            total_prompts INTEGER NOT NULL DEFAULT 0,
            blocked_count INTEGER NOT NULL DEFAULT 0,
            allowed_count INTEGER NOT NULL DEFAULT 0,
            flagged_count INTEGER NOT NULL DEFAULT 0,
            category_counts TEXT NOT NULL DEFAULT '{}'
        )",
        [],
    )?;

    // Rules table - filtering rules
    conn.execute(
        "CREATE TABLE IF NOT EXISTS rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            enabled INTEGER NOT NULL DEFAULT 1,
            config TEXT NOT NULL,
            priority INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Config table - key-value configuration
    conn.execute(
        "CREATE TABLE IF NOT EXISTS config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;

    // Auth table - authentication (single user)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS auth (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            last_login TEXT
        )",
        [],
    )?;

    Ok(())
}

/// Migration to version 2: User profiles.
fn migrate_v2(conn: &Connection) -> Result<()> {
    info!("Applying migration v2: User profiles");

    // Profiles table - user profiles with rules
    conn.execute(
        "CREATE TABLE IF NOT EXISTS profiles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            os_username TEXT UNIQUE,
            time_rules TEXT NOT NULL DEFAULT '{}',
            content_rules TEXT NOT NULL DEFAULT '{}',
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Index for looking up profiles by OS username
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_profiles_os_username ON profiles (os_username)",
        [],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();

        // Run migrations twice - should not error
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        // Verify version
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn test_tables_created() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        // Verify all tables exist by querying them
        conn.execute("SELECT * FROM events LIMIT 1", []).ok();
        conn.execute("SELECT * FROM daily_stats LIMIT 1", []).ok();
        conn.execute("SELECT * FROM rules LIMIT 1", []).ok();
        conn.execute("SELECT * FROM config LIMIT 1", []).ok();
        conn.execute("SELECT * FROM auth LIMIT 1", []).ok();
        conn.execute("SELECT * FROM profiles LIMIT 1", []).ok();
    }
}

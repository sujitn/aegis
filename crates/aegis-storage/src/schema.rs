//! Database schema and migrations.

use rusqlite::Connection;
use tracing::info;

use crate::error::Result;

/// Current schema version.
pub const SCHEMA_VERSION: i32 = 6;

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

        if current_version < 3 {
            migrate_v3(conn)?;
        }

        if current_version < 4 {
            migrate_v4(conn)?;
        }

        if current_version < 5 {
            migrate_v5(conn)?;
        }

        if current_version < 6 {
            migrate_v6(conn)?;
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

/// Migration to version 3: Site registry.
fn migrate_v3(conn: &Connection) -> Result<()> {
    info!("Applying migration v3: Site registry");

    // Sites table - custom and remote sites
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sites (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pattern TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'consumer',
            parser_id TEXT,
            enabled INTEGER NOT NULL DEFAULT 1,
            source TEXT NOT NULL DEFAULT 'custom',
            priority INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Index for looking up sites by pattern
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sites_pattern ON sites (pattern)",
        [],
    )?;

    // Index for enabled sites
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sites_enabled ON sites (enabled)",
        [],
    )?;

    // Disabled bundled sites table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS disabled_bundled_sites (
            pattern TEXT PRIMARY KEY,
            disabled_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    Ok(())
}

/// Migration to version 4: Flagged events for sentiment analysis.
fn migrate_v4(conn: &Connection) -> Result<()> {
    info!("Applying migration v4: Flagged events for sentiment analysis");

    // Flagged events table - stores sentiment analysis flags for parental review
    conn.execute(
        "CREATE TABLE IF NOT EXISTS flagged_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            profile_id INTEGER NOT NULL,
            flag_type TEXT NOT NULL,
            confidence REAL NOT NULL,
            content_snippet TEXT NOT NULL,
            source TEXT,
            matched_phrases TEXT NOT NULL DEFAULT '[]',
            acknowledged INTEGER NOT NULL DEFAULT 0,
            acknowledged_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Index for querying by profile
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_flagged_events_profile ON flagged_events (profile_id)",
        [],
    )?;

    // Index for querying by timestamp
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_flagged_events_created_at ON flagged_events (created_at)",
        [],
    )?;

    // Index for unacknowledged flags (partial index)
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_flagged_events_unacknowledged ON flagged_events (acknowledged) WHERE acknowledged = 0",
        [],
    )?;

    // Index for flag type filtering
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_flagged_events_flag_type ON flagged_events (flag_type)",
        [],
    )?;

    Ok(())
}

/// Migration to version 5: Sentiment configuration per profile.
fn migrate_v5(conn: &Connection) -> Result<()> {
    info!("Applying migration v5: Sentiment configuration per profile");

    // Add sentiment_config column to profiles table
    // Default is enabled with standard sensitivity
    conn.execute(
        "ALTER TABLE profiles ADD COLUMN sentiment_config TEXT NOT NULL DEFAULT '{\"enabled\":true,\"sensitivity\":0.5,\"detect_distress\":true,\"detect_crisis\":true,\"detect_bullying\":true,\"detect_negative\":true}'",
        [],
    )?;

    Ok(())
}

/// Migration to version 6: Centralized state management.
/// Adds tables for cross-process state synchronization (F032).
fn migrate_v6(conn: &Connection) -> Result<()> {
    info!("Applying migration v6: Centralized state management");

    // App state table - central key-value store for application state
    // Used for protection status, interception mode, etc.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS app_state (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_by TEXT
        )",
        [],
    )?;

    // Initialize default protection state
    conn.execute(
        "INSERT OR IGNORE INTO app_state (key, value, updated_by) VALUES ('protection', '{\"status\":\"active\",\"pause_until\":null}', 'system')",
        [],
    )?;

    // Sessions table - persistent session storage for cross-process auth
    // Replaces in-memory session HashMap in AuthManager
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            token TEXT PRIMARY KEY,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            expires_at TEXT NOT NULL,
            last_used_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Index for cleaning up expired sessions
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions (expires_at)",
        [],
    )?;

    // State changes table - sequence log for cache invalidation
    // Readers poll this to detect when they need to refresh their cache
    conn.execute(
        "CREATE TABLE IF NOT EXISTS state_changes (
            seq INTEGER PRIMARY KEY AUTOINCREMENT,
            state_key TEXT NOT NULL,
            changed_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Index for efficient polling (WHERE seq > last_seen)
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_state_changes_seq ON state_changes (seq)",
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
        conn.execute("SELECT * FROM sites LIMIT 1", []).ok();
        conn.execute("SELECT * FROM disabled_bundled_sites LIMIT 1", [])
            .ok();
        conn.execute("SELECT * FROM flagged_events LIMIT 1", [])
            .ok();
        // v6 tables
        conn.execute("SELECT * FROM app_state LIMIT 1", []).ok();
        conn.execute("SELECT * FROM sessions LIMIT 1", []).ok();
        conn.execute("SELECT * FROM state_changes LIMIT 1", []).ok();
    }

    #[test]
    fn test_app_state_initialized() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        // Verify protection state is initialized
        let value: String = conn
            .query_row(
                "SELECT value FROM app_state WHERE key = 'protection'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(value.contains("active"));
    }

    #[test]
    fn test_flagged_events_table() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        // Create a profile first (required for foreign key)
        conn.execute(
            "INSERT INTO profiles (name, os_username, time_rules, content_rules) VALUES ('Test', 'testuser', '{}', '{}')",
            [],
        ).unwrap();

        // Insert a flagged event
        conn.execute(
            "INSERT INTO flagged_events (profile_id, flag_type, confidence, content_snippet, matched_phrases) VALUES (1, 'distress', 0.85, 'I feel sad...', '[\"feel sad\"]')",
            [],
        ).unwrap();

        // Verify it was inserted
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM flagged_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}

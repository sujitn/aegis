//! Database connection pool.
//!
//! Uses a simple Mutex-protected connection for thread safety.
//! For a local-only application, this is sufficient and simpler than r2d2.

use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::error::{Result, StorageError};
use crate::schema::run_migrations;

/// Thread-safe database connection pool.
///
/// This is a simple wrapper around a Mutex-protected Connection.
/// For a local desktop application, this provides adequate concurrency.
#[derive(Clone)]
pub struct ConnectionPool {
    conn: Arc<Mutex<Connection>>,
}

impl ConnectionPool {
    /// Create a new connection pool with a file-based database.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::setup_connection(&conn)?;
        run_migrations(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create a new connection pool with an in-memory database.
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::setup_connection(&conn)?;
        run_migrations(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Get a connection from the pool.
    pub fn get(&self) -> Result<PooledConnection<'_>> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| StorageError::Config("Connection pool poisoned".to_string()))?;

        Ok(PooledConnection { guard })
    }

    /// Setup connection pragmas for performance and safety.
    fn setup_connection(conn: &Connection) -> Result<()> {
        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        // Use WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;

        // Sync mode for durability vs performance balance
        conn.execute_batch("PRAGMA synchronous = NORMAL;")?;

        // Cache size (negative = KB, positive = pages)
        conn.execute_batch("PRAGMA cache_size = -2000;")?;

        Ok(())
    }
}

/// A connection borrowed from the pool.
pub struct PooledConnection<'a> {
    guard: MutexGuard<'a, Connection>,
}

impl<'a> std::ops::Deref for PooledConnection<'a> {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_pool() {
        let pool = ConnectionPool::in_memory().unwrap();
        let conn = pool.get().unwrap();

        // Verify we can query
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_pool_is_clone() {
        let pool1 = ConnectionPool::in_memory().unwrap();
        let pool2 = pool1.clone();

        // Both should work
        let _ = pool1.get().unwrap();
        let _ = pool2.get().unwrap();
    }

    #[test]
    fn test_multiple_gets() {
        let pool = ConnectionPool::in_memory().unwrap();

        // Sequential gets should work
        {
            let _conn = pool.get().unwrap();
        }
        {
            let _conn = pool.get().unwrap();
        }
    }
}

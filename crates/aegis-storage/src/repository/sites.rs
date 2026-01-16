//! Site repository.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

use crate::error::{Result, StorageError};
use crate::models::{DisabledBundledSite, NewSite, Site};

/// Repository for site operations.
pub struct SiteRepo;

impl SiteRepo {
    /// Insert a new site.
    pub fn insert(conn: &Connection, site: NewSite) -> Result<i64> {
        conn.execute(
            "INSERT INTO sites (pattern, name, category, parser_id, enabled, source, priority)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                site.pattern,
                site.name,
                site.category,
                site.parser_id,
                site.enabled as i32,
                site.source,
                site.priority
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get a site by ID.
    pub fn get_by_id(conn: &Connection, id: i64) -> Result<Option<Site>> {
        let mut stmt = conn.prepare(
            "SELECT id, pattern, name, category, parser_id, enabled, source, priority, created_at, updated_at
             FROM sites WHERE id = ?1",
        )?;

        let site = stmt
            .query_row([id], |row| {
                Ok(Site {
                    id: row.get(0)?,
                    pattern: row.get(1)?,
                    name: row.get(2)?,
                    category: row.get(3)?,
                    parser_id: row.get(4)?,
                    enabled: row.get::<_, i32>(5)? != 0,
                    source: row.get(6)?,
                    priority: row.get(7)?,
                    created_at: parse_datetime(&row.get::<_, String>(8)?),
                    updated_at: parse_datetime(&row.get::<_, String>(9)?),
                })
            })
            .ok();

        Ok(site)
    }

    /// Get a site by pattern.
    pub fn get_by_pattern(conn: &Connection, pattern: &str) -> Result<Option<Site>> {
        let mut stmt = conn.prepare(
            "SELECT id, pattern, name, category, parser_id, enabled, source, priority, created_at, updated_at
             FROM sites WHERE pattern = ?1",
        )?;

        let site = stmt
            .query_row([pattern], |row| {
                Ok(Site {
                    id: row.get(0)?,
                    pattern: row.get(1)?,
                    name: row.get(2)?,
                    category: row.get(3)?,
                    parser_id: row.get(4)?,
                    enabled: row.get::<_, i32>(5)? != 0,
                    source: row.get(6)?,
                    priority: row.get(7)?,
                    created_at: parse_datetime(&row.get::<_, String>(8)?),
                    updated_at: parse_datetime(&row.get::<_, String>(9)?),
                })
            })
            .ok();

        Ok(site)
    }

    /// Get all sites.
    pub fn get_all(conn: &Connection) -> Result<Vec<Site>> {
        let mut stmt = conn.prepare(
            "SELECT id, pattern, name, category, parser_id, enabled, source, priority, created_at, updated_at
             FROM sites ORDER BY priority DESC, name ASC",
        )?;

        let sites = stmt
            .query_map([], |row| {
                Ok(Site {
                    id: row.get(0)?,
                    pattern: row.get(1)?,
                    name: row.get(2)?,
                    category: row.get(3)?,
                    parser_id: row.get(4)?,
                    enabled: row.get::<_, i32>(5)? != 0,
                    source: row.get(6)?,
                    priority: row.get(7)?,
                    created_at: parse_datetime(&row.get::<_, String>(8)?),
                    updated_at: parse_datetime(&row.get::<_, String>(9)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sites)
    }

    /// Get all enabled sites.
    pub fn get_enabled(conn: &Connection) -> Result<Vec<Site>> {
        let mut stmt = conn.prepare(
            "SELECT id, pattern, name, category, parser_id, enabled, source, priority, created_at, updated_at
             FROM sites WHERE enabled = 1 ORDER BY priority DESC, name ASC",
        )?;

        let sites = stmt
            .query_map([], |row| {
                Ok(Site {
                    id: row.get(0)?,
                    pattern: row.get(1)?,
                    name: row.get(2)?,
                    category: row.get(3)?,
                    parser_id: row.get(4)?,
                    enabled: row.get::<_, i32>(5)? != 0,
                    source: row.get(6)?,
                    priority: row.get(7)?,
                    created_at: parse_datetime(&row.get::<_, String>(8)?),
                    updated_at: parse_datetime(&row.get::<_, String>(9)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sites)
    }

    /// Get sites by source.
    pub fn get_by_source(conn: &Connection, source: &str) -> Result<Vec<Site>> {
        let mut stmt = conn.prepare(
            "SELECT id, pattern, name, category, parser_id, enabled, source, priority, created_at, updated_at
             FROM sites WHERE source = ?1 ORDER BY priority DESC, name ASC",
        )?;

        let sites = stmt
            .query_map([source], |row| {
                Ok(Site {
                    id: row.get(0)?,
                    pattern: row.get(1)?,
                    name: row.get(2)?,
                    category: row.get(3)?,
                    parser_id: row.get(4)?,
                    enabled: row.get::<_, i32>(5)? != 0,
                    source: row.get(6)?,
                    priority: row.get(7)?,
                    created_at: parse_datetime(&row.get::<_, String>(8)?),
                    updated_at: parse_datetime(&row.get::<_, String>(9)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sites)
    }

    /// Update a site.
    pub fn update(conn: &Connection, id: i64, site: NewSite) -> Result<()> {
        let updated = conn.execute(
            "UPDATE sites SET pattern = ?1, name = ?2, category = ?3, parser_id = ?4,
             enabled = ?5, source = ?6, priority = ?7, updated_at = datetime('now') WHERE id = ?8",
            params![
                site.pattern,
                site.name,
                site.category,
                site.parser_id,
                site.enabled as i32,
                site.source,
                site.priority,
                id
            ],
        )?;

        if updated == 0 {
            return Err(StorageError::NotFound(format!("Site with id {}", id)));
        }

        Ok(())
    }

    /// Enable or disable a site.
    pub fn set_enabled(conn: &Connection, id: i64, enabled: bool) -> Result<()> {
        let updated = conn.execute(
            "UPDATE sites SET enabled = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![enabled as i32, id],
        )?;

        if updated == 0 {
            return Err(StorageError::NotFound(format!("Site with id {}", id)));
        }

        Ok(())
    }

    /// Enable or disable a site by pattern.
    pub fn set_enabled_by_pattern(conn: &Connection, pattern: &str, enabled: bool) -> Result<()> {
        let updated = conn.execute(
            "UPDATE sites SET enabled = ?1, updated_at = datetime('now') WHERE pattern = ?2",
            params![enabled as i32, pattern],
        )?;

        if updated == 0 {
            return Err(StorageError::NotFound(format!(
                "Site with pattern {}",
                pattern
            )));
        }

        Ok(())
    }

    /// Delete a site.
    pub fn delete(conn: &Connection, id: i64) -> Result<()> {
        let deleted = conn.execute("DELETE FROM sites WHERE id = ?1", [id])?;

        if deleted == 0 {
            return Err(StorageError::NotFound(format!("Site with id {}", id)));
        }

        Ok(())
    }

    /// Delete a site by pattern.
    pub fn delete_by_pattern(conn: &Connection, pattern: &str) -> Result<()> {
        let deleted = conn.execute("DELETE FROM sites WHERE pattern = ?1", [pattern])?;

        if deleted == 0 {
            return Err(StorageError::NotFound(format!(
                "Site with pattern {}",
                pattern
            )));
        }

        Ok(())
    }

    /// Count total sites.
    pub fn count(conn: &Connection) -> Result<i64> {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM sites", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Upsert a site (insert or update by pattern).
    pub fn upsert(conn: &Connection, site: NewSite) -> Result<i64> {
        conn.execute(
            "INSERT INTO sites (pattern, name, category, parser_id, enabled, source, priority)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(pattern) DO UPDATE SET
                name = excluded.name,
                category = excluded.category,
                parser_id = excluded.parser_id,
                enabled = excluded.enabled,
                source = excluded.source,
                priority = excluded.priority,
                updated_at = datetime('now')",
            params![
                site.pattern,
                site.name,
                site.category,
                site.parser_id,
                site.enabled as i32,
                site.source,
                site.priority
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }
}

/// Repository for disabled bundled site operations.
pub struct DisabledBundledRepo;

impl DisabledBundledRepo {
    /// Add a disabled bundled site pattern.
    pub fn add(conn: &Connection, pattern: &str) -> Result<()> {
        conn.execute(
            "INSERT OR IGNORE INTO disabled_bundled_sites (pattern) VALUES (?1)",
            [pattern],
        )?;
        Ok(())
    }

    /// Remove a disabled bundled site pattern (re-enable it).
    pub fn remove(conn: &Connection, pattern: &str) -> Result<()> {
        conn.execute(
            "DELETE FROM disabled_bundled_sites WHERE pattern = ?1",
            [pattern],
        )?;
        Ok(())
    }

    /// Check if a pattern is disabled.
    pub fn is_disabled(conn: &Connection, pattern: &str) -> Result<bool> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM disabled_bundled_sites WHERE pattern = ?1",
            [pattern],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Get all disabled bundled site patterns.
    pub fn get_all(conn: &Connection) -> Result<Vec<DisabledBundledSite>> {
        let mut stmt = conn
            .prepare("SELECT pattern, disabled_at FROM disabled_bundled_sites ORDER BY pattern")?;

        let patterns = stmt
            .query_map([], |row| {
                Ok(DisabledBundledSite {
                    pattern: row.get(0)?,
                    disabled_at: parse_datetime(&row.get::<_, String>(1)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(patterns)
    }

    /// Get all disabled patterns as a simple list.
    pub fn get_patterns(conn: &Connection) -> Result<Vec<String>> {
        let mut stmt =
            conn.prepare("SELECT pattern FROM disabled_bundled_sites ORDER BY pattern")?;

        let patterns = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(patterns)
    }

    /// Clear all disabled bundled sites.
    pub fn clear(conn: &Connection) -> Result<()> {
        conn.execute("DELETE FROM disabled_bundled_sites", [])?;
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

    // ==================== SiteRepo Tests ====================

    #[test]
    fn test_insert_and_get_site() {
        let conn = setup_db();

        let site = NewSite {
            pattern: "custom.example.com".to_string(),
            name: "Custom Example".to_string(),
            category: "enterprise".to_string(),
            parser_id: Some("openai_json".to_string()),
            enabled: true,
            source: "custom".to_string(),
            priority: 100,
        };

        let id = SiteRepo::insert(&conn, site).unwrap();
        let retrieved = SiteRepo::get_by_id(&conn, id).unwrap().unwrap();

        assert_eq!(retrieved.pattern, "custom.example.com");
        assert_eq!(retrieved.name, "Custom Example");
        assert_eq!(retrieved.category, "enterprise");
        assert_eq!(retrieved.parser_id, Some("openai_json".to_string()));
        assert!(retrieved.enabled);
        assert_eq!(retrieved.source, "custom");
        assert_eq!(retrieved.priority, 100);
    }

    #[test]
    fn test_get_by_pattern() {
        let conn = setup_db();

        let site = NewSite {
            pattern: "api.myai.com".to_string(),
            name: "My AI".to_string(),
            category: "api".to_string(),
            parser_id: None,
            enabled: true,
            source: "custom".to_string(),
            priority: 50,
        };

        SiteRepo::insert(&conn, site).unwrap();
        let retrieved = SiteRepo::get_by_pattern(&conn, "api.myai.com")
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.name, "My AI");
    }

    #[test]
    fn test_get_all_sites() {
        let conn = setup_db();

        SiteRepo::insert(
            &conn,
            NewSite {
                pattern: "site1.com".to_string(),
                name: "Site 1".to_string(),
                category: "consumer".to_string(),
                parser_id: None,
                enabled: true,
                source: "custom".to_string(),
                priority: 10,
            },
        )
        .unwrap();

        SiteRepo::insert(
            &conn,
            NewSite {
                pattern: "site2.com".to_string(),
                name: "Site 2".to_string(),
                category: "api".to_string(),
                parser_id: None,
                enabled: true,
                source: "custom".to_string(),
                priority: 20,
            },
        )
        .unwrap();

        let all = SiteRepo::get_all(&conn).unwrap();
        assert_eq!(all.len(), 2);
        // Should be ordered by priority DESC
        assert_eq!(all[0].name, "Site 2"); // Higher priority first
    }

    #[test]
    fn test_get_enabled_sites() {
        let conn = setup_db();

        SiteRepo::insert(
            &conn,
            NewSite {
                pattern: "enabled.com".to_string(),
                name: "Enabled".to_string(),
                category: "consumer".to_string(),
                parser_id: None,
                enabled: true,
                source: "custom".to_string(),
                priority: 0,
            },
        )
        .unwrap();

        SiteRepo::insert(
            &conn,
            NewSite {
                pattern: "disabled.com".to_string(),
                name: "Disabled".to_string(),
                category: "consumer".to_string(),
                parser_id: None,
                enabled: false,
                source: "custom".to_string(),
                priority: 0,
            },
        )
        .unwrap();

        let enabled = SiteRepo::get_enabled(&conn).unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "Enabled");
    }

    #[test]
    fn test_get_by_source() {
        let conn = setup_db();

        SiteRepo::insert(
            &conn,
            NewSite {
                pattern: "custom1.com".to_string(),
                name: "Custom 1".to_string(),
                category: "consumer".to_string(),
                parser_id: None,
                enabled: true,
                source: "custom".to_string(),
                priority: 0,
            },
        )
        .unwrap();

        SiteRepo::insert(
            &conn,
            NewSite {
                pattern: "remote1.com".to_string(),
                name: "Remote 1".to_string(),
                category: "consumer".to_string(),
                parser_id: None,
                enabled: true,
                source: "remote".to_string(),
                priority: 0,
            },
        )
        .unwrap();

        let custom = SiteRepo::get_by_source(&conn, "custom").unwrap();
        assert_eq!(custom.len(), 1);
        assert_eq!(custom[0].name, "Custom 1");

        let remote = SiteRepo::get_by_source(&conn, "remote").unwrap();
        assert_eq!(remote.len(), 1);
        assert_eq!(remote[0].name, "Remote 1");
    }

    #[test]
    fn test_update_site() {
        let conn = setup_db();

        let site = NewSite {
            pattern: "original.com".to_string(),
            name: "Original".to_string(),
            category: "consumer".to_string(),
            parser_id: None,
            enabled: true,
            source: "custom".to_string(),
            priority: 0,
        };

        let id = SiteRepo::insert(&conn, site).unwrap();

        SiteRepo::update(
            &conn,
            id,
            NewSite {
                pattern: "original.com".to_string(),
                name: "Updated".to_string(),
                category: "api".to_string(),
                parser_id: Some("new_parser".to_string()),
                enabled: false,
                source: "custom".to_string(),
                priority: 50,
            },
        )
        .unwrap();

        let updated = SiteRepo::get_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.category, "api");
        assert_eq!(updated.parser_id, Some("new_parser".to_string()));
        assert!(!updated.enabled);
        assert_eq!(updated.priority, 50);
    }

    #[test]
    fn test_set_enabled() {
        let conn = setup_db();

        let site = NewSite {
            pattern: "toggle.com".to_string(),
            name: "Toggle".to_string(),
            category: "consumer".to_string(),
            parser_id: None,
            enabled: true,
            source: "custom".to_string(),
            priority: 0,
        };

        let id = SiteRepo::insert(&conn, site).unwrap();
        assert!(SiteRepo::get_by_id(&conn, id).unwrap().unwrap().enabled);

        SiteRepo::set_enabled(&conn, id, false).unwrap();
        assert!(!SiteRepo::get_by_id(&conn, id).unwrap().unwrap().enabled);

        SiteRepo::set_enabled(&conn, id, true).unwrap();
        assert!(SiteRepo::get_by_id(&conn, id).unwrap().unwrap().enabled);
    }

    #[test]
    fn test_set_enabled_by_pattern() {
        let conn = setup_db();

        let site = NewSite {
            pattern: "bypattern.com".to_string(),
            name: "By Pattern".to_string(),
            category: "consumer".to_string(),
            parser_id: None,
            enabled: true,
            source: "custom".to_string(),
            priority: 0,
        };

        SiteRepo::insert(&conn, site).unwrap();

        SiteRepo::set_enabled_by_pattern(&conn, "bypattern.com", false).unwrap();
        let site = SiteRepo::get_by_pattern(&conn, "bypattern.com")
            .unwrap()
            .unwrap();
        assert!(!site.enabled);
    }

    #[test]
    fn test_delete_site() {
        let conn = setup_db();

        let site = NewSite {
            pattern: "todelete.com".to_string(),
            name: "To Delete".to_string(),
            category: "consumer".to_string(),
            parser_id: None,
            enabled: true,
            source: "custom".to_string(),
            priority: 0,
        };

        let id = SiteRepo::insert(&conn, site).unwrap();
        assert!(SiteRepo::get_by_id(&conn, id).unwrap().is_some());

        SiteRepo::delete(&conn, id).unwrap();
        assert!(SiteRepo::get_by_id(&conn, id).unwrap().is_none());
    }

    #[test]
    fn test_delete_by_pattern() {
        let conn = setup_db();

        let site = NewSite {
            pattern: "deletebypattern.com".to_string(),
            name: "Delete By Pattern".to_string(),
            category: "consumer".to_string(),
            parser_id: None,
            enabled: true,
            source: "custom".to_string(),
            priority: 0,
        };

        SiteRepo::insert(&conn, site).unwrap();
        assert!(SiteRepo::get_by_pattern(&conn, "deletebypattern.com")
            .unwrap()
            .is_some());

        SiteRepo::delete_by_pattern(&conn, "deletebypattern.com").unwrap();
        assert!(SiteRepo::get_by_pattern(&conn, "deletebypattern.com")
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_count_sites() {
        let conn = setup_db();
        assert_eq!(SiteRepo::count(&conn).unwrap(), 0);

        SiteRepo::insert(
            &conn,
            NewSite {
                pattern: "count1.com".to_string(),
                name: "Count 1".to_string(),
                category: "consumer".to_string(),
                parser_id: None,
                enabled: true,
                source: "custom".to_string(),
                priority: 0,
            },
        )
        .unwrap();

        assert_eq!(SiteRepo::count(&conn).unwrap(), 1);
    }

    #[test]
    fn test_upsert_insert() {
        let conn = setup_db();

        let site = NewSite {
            pattern: "upsert.com".to_string(),
            name: "Upsert".to_string(),
            category: "consumer".to_string(),
            parser_id: None,
            enabled: true,
            source: "custom".to_string(),
            priority: 0,
        };

        SiteRepo::upsert(&conn, site).unwrap();
        let retrieved = SiteRepo::get_by_pattern(&conn, "upsert.com")
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.name, "Upsert");
    }

    #[test]
    fn test_upsert_update() {
        let conn = setup_db();

        // First insert
        SiteRepo::upsert(
            &conn,
            NewSite {
                pattern: "upsert2.com".to_string(),
                name: "Original".to_string(),
                category: "consumer".to_string(),
                parser_id: None,
                enabled: true,
                source: "custom".to_string(),
                priority: 0,
            },
        )
        .unwrap();

        // Then upsert (update)
        SiteRepo::upsert(
            &conn,
            NewSite {
                pattern: "upsert2.com".to_string(),
                name: "Updated".to_string(),
                category: "api".to_string(),
                parser_id: Some("parser".to_string()),
                enabled: false,
                source: "custom".to_string(),
                priority: 100,
            },
        )
        .unwrap();

        let retrieved = SiteRepo::get_by_pattern(&conn, "upsert2.com")
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.name, "Updated");
        assert_eq!(retrieved.category, "api");
        assert_eq!(retrieved.priority, 100);
        // Should only have 1 entry
        assert_eq!(SiteRepo::count(&conn).unwrap(), 1);
    }

    // ==================== DisabledBundledRepo Tests ====================

    #[test]
    fn test_add_disabled_bundled() {
        let conn = setup_db();

        DisabledBundledRepo::add(&conn, "api.openai.com").unwrap();
        assert!(DisabledBundledRepo::is_disabled(&conn, "api.openai.com").unwrap());
    }

    #[test]
    fn test_remove_disabled_bundled() {
        let conn = setup_db();

        DisabledBundledRepo::add(&conn, "api.openai.com").unwrap();
        assert!(DisabledBundledRepo::is_disabled(&conn, "api.openai.com").unwrap());

        DisabledBundledRepo::remove(&conn, "api.openai.com").unwrap();
        assert!(!DisabledBundledRepo::is_disabled(&conn, "api.openai.com").unwrap());
    }

    #[test]
    fn test_is_disabled_false() {
        let conn = setup_db();
        assert!(!DisabledBundledRepo::is_disabled(&conn, "not.disabled.com").unwrap());
    }

    #[test]
    fn test_get_all_disabled_bundled() {
        let conn = setup_db();

        DisabledBundledRepo::add(&conn, "site1.com").unwrap();
        DisabledBundledRepo::add(&conn, "site2.com").unwrap();

        let all = DisabledBundledRepo::get_all(&conn).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_get_patterns() {
        let conn = setup_db();

        DisabledBundledRepo::add(&conn, "alpha.com").unwrap();
        DisabledBundledRepo::add(&conn, "beta.com").unwrap();

        let patterns = DisabledBundledRepo::get_patterns(&conn).unwrap();
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0], "alpha.com");
        assert_eq!(patterns[1], "beta.com");
    }

    #[test]
    fn test_clear_disabled_bundled() {
        let conn = setup_db();

        DisabledBundledRepo::add(&conn, "site1.com").unwrap();
        DisabledBundledRepo::add(&conn, "site2.com").unwrap();

        DisabledBundledRepo::clear(&conn).unwrap();
        assert_eq!(DisabledBundledRepo::get_all(&conn).unwrap().len(), 0);
    }

    #[test]
    fn test_add_duplicate_disabled() {
        let conn = setup_db();

        // Adding same pattern twice should not error (INSERT OR IGNORE)
        DisabledBundledRepo::add(&conn, "duplicate.com").unwrap();
        DisabledBundledRepo::add(&conn, "duplicate.com").unwrap();

        let patterns = DisabledBundledRepo::get_patterns(&conn).unwrap();
        assert_eq!(patterns.len(), 1);
    }
}

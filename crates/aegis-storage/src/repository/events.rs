//! Events repository.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};

use crate::error::Result;
use crate::models::{Action, Event, NewEvent};

/// Maximum preview length in characters.
const PREVIEW_MAX_LEN: usize = 50;

/// Repository for event operations.
pub struct EventsRepo;

impl EventsRepo {
    /// Insert a new event.
    pub fn insert(conn: &Connection, event: NewEvent) -> Result<i64> {
        conn.execute(
            "INSERT INTO events (prompt_hash, preview, category, confidence, action, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.prompt_hash,
                event.preview,
                event.category.map(|c| format!("{:?}", c).to_lowercase()),
                event.confidence,
                event.action.as_str(),
                event.source,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get an event by ID.
    pub fn get_by_id(conn: &Connection, id: i64) -> Result<Option<Event>> {
        let mut stmt = conn.prepare(
            "SELECT id, prompt_hash, preview, category, confidence, action, source, created_at
             FROM events WHERE id = ?1",
        )?;

        let event = stmt
            .query_row([id], |row| {
                Ok(Event {
                    id: row.get(0)?,
                    prompt_hash: row.get(1)?,
                    preview: row.get(2)?,
                    category: row
                        .get::<_, Option<String>>(3)?
                        .and_then(|s| parse_category(&s)),
                    confidence: row.get(4)?,
                    action: row
                        .get::<_, String>(5)
                        .ok()
                        .and_then(|s| Action::parse(&s))
                        .unwrap_or(Action::Allowed),
                    source: row.get(6)?,
                    created_at: parse_datetime(&row.get::<_, String>(7)?),
                })
            })
            .ok();

        Ok(event)
    }

    /// Get recent events with pagination.
    pub fn get_recent(conn: &Connection, limit: i64, offset: i64) -> Result<Vec<Event>> {
        let mut stmt = conn.prepare(
            "SELECT id, prompt_hash, preview, category, confidence, action, source, created_at
             FROM events ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
        )?;

        let events = stmt
            .query_map([limit, offset], |row| {
                Ok(Event {
                    id: row.get(0)?,
                    prompt_hash: row.get(1)?,
                    preview: row.get(2)?,
                    category: row
                        .get::<_, Option<String>>(3)?
                        .and_then(|s| parse_category(&s)),
                    confidence: row.get(4)?,
                    action: row
                        .get::<_, String>(5)
                        .ok()
                        .and_then(|s| Action::parse(&s))
                        .unwrap_or(Action::Allowed),
                    source: row.get(6)?,
                    created_at: parse_datetime(&row.get::<_, String>(7)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(events)
    }

    /// Get events by action type.
    pub fn get_by_action(
        conn: &Connection,
        action: Action,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Event>> {
        let mut stmt = conn.prepare(
            "SELECT id, prompt_hash, preview, category, confidence, action, source, created_at
             FROM events WHERE action = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
        )?;

        let events = stmt
            .query_map(params![action.as_str(), limit, offset], |row| {
                Ok(Event {
                    id: row.get(0)?,
                    prompt_hash: row.get(1)?,
                    preview: row.get(2)?,
                    category: row
                        .get::<_, Option<String>>(3)?
                        .and_then(|s| parse_category(&s)),
                    confidence: row.get(4)?,
                    action: row
                        .get::<_, String>(5)
                        .ok()
                        .and_then(|s| Action::parse(&s))
                        .unwrap_or(Action::Allowed),
                    source: row.get(6)?,
                    created_at: parse_datetime(&row.get::<_, String>(7)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(events)
    }

    /// Count total events.
    pub fn count(conn: &Connection) -> Result<i64> {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Count events by action.
    pub fn count_by_action(conn: &Connection, action: Action) -> Result<i64> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM events WHERE action = ?1",
            [action.as_str()],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Delete events older than a given date.
    pub fn delete_older_than(conn: &Connection, before: DateTime<Utc>) -> Result<i64> {
        let deleted = conn.execute(
            "DELETE FROM events WHERE created_at < ?1",
            [before.to_rfc3339()],
        )?;
        Ok(deleted as i64)
    }
}

/// Hash a prompt using SHA-256.
pub fn hash_prompt(prompt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prompt.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Create a preview from a prompt (truncated and redacted).
pub fn create_preview(prompt: &str) -> String {
    let cleaned: String = prompt
        .chars()
        .filter(|c| !c.is_control())
        .take(PREVIEW_MAX_LEN)
        .collect();

    if prompt.len() > PREVIEW_MAX_LEN {
        format!("{}...", cleaned)
    } else {
        cleaned
    }
}

/// Parse a category from string.
fn parse_category(s: &str) -> Option<aegis_core::classifier::Category> {
    use aegis_core::classifier::Category;
    match s {
        "violence" => Some(Category::Violence),
        "selfharm" | "self_harm" => Some(Category::SelfHarm),
        "adult" => Some(Category::Adult),
        "jailbreak" => Some(Category::Jailbreak),
        "hate" => Some(Category::Hate),
        "illegal" => Some(Category::Illegal),
        _ => None,
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

// We need hex encoding for the hash
mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        let bytes = bytes.as_ref();
        let mut hex = String::with_capacity(bytes.len() * 2);
        for &byte in bytes {
            hex.push(HEX_CHARS[(byte >> 4) as usize] as char);
            hex.push(HEX_CHARS[(byte & 0xf) as usize] as char);
        }
        hex
    }
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
    fn test_insert_and_get_event() {
        let conn = setup_db();

        let event = NewEvent {
            prompt_hash: hash_prompt("test prompt"),
            preview: create_preview("test prompt"),
            category: Some(aegis_core::classifier::Category::Violence),
            confidence: Some(0.95),
            action: Action::Blocked,
            source: Some("test".to_string()),
        };

        let id = EventsRepo::insert(&conn, event).unwrap();
        let retrieved = EventsRepo::get_by_id(&conn, id).unwrap().unwrap();

        assert_eq!(retrieved.preview, "test prompt");
        assert_eq!(retrieved.action, Action::Blocked);
        assert!(retrieved.confidence.unwrap() > 0.9);
    }

    #[test]
    fn test_get_recent_events() {
        let conn = setup_db();

        for i in 0..5 {
            let event = NewEvent {
                prompt_hash: hash_prompt(&format!("prompt {}", i)),
                preview: create_preview(&format!("prompt {}", i)),
                category: None,
                confidence: None,
                action: Action::Allowed,
                source: None,
            };
            EventsRepo::insert(&conn, event).unwrap();
        }

        let events = EventsRepo::get_recent(&conn, 3, 0).unwrap();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_count_events() {
        let conn = setup_db();

        for _ in 0..3 {
            let event = NewEvent {
                prompt_hash: hash_prompt("test"),
                preview: "test".to_string(),
                category: None,
                confidence: None,
                action: Action::Blocked,
                source: None,
            };
            EventsRepo::insert(&conn, event).unwrap();
        }

        assert_eq!(EventsRepo::count(&conn).unwrap(), 3);
        assert_eq!(
            EventsRepo::count_by_action(&conn, Action::Blocked).unwrap(),
            3
        );
        assert_eq!(
            EventsRepo::count_by_action(&conn, Action::Allowed).unwrap(),
            0
        );
    }

    #[test]
    fn test_hash_prompt() {
        let hash1 = hash_prompt("hello");
        let hash2 = hash_prompt("hello");
        let hash3 = hash_prompt("world");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA-256 produces 64 hex chars
    }

    #[test]
    fn test_create_preview() {
        assert_eq!(create_preview("short"), "short");
        assert_eq!(
            create_preview("a".repeat(100).as_str()),
            format!("{}...", "a".repeat(50))
        );
    }
}

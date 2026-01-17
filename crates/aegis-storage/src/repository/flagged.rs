//! Flagged events repository for sentiment analysis.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

use crate::error::Result;
use crate::models::{
    FlaggedEvent, FlaggedEventFilter, FlaggedEventStats, FlaggedTypeCounts, NewFlaggedEvent,
};

/// Maximum content snippet length in characters.
const SNIPPET_MAX_LEN: usize = 200;

/// Repository for flagged event operations.
pub struct FlaggedEventsRepo;

impl FlaggedEventsRepo {
    /// Insert a new flagged event.
    pub fn insert(conn: &Connection, event: NewFlaggedEvent) -> Result<i64> {
        let matched_phrases_json =
            serde_json::to_string(&event.matched_phrases).unwrap_or_else(|_| "[]".to_string());

        conn.execute(
            "INSERT INTO flagged_events (profile_id, flag_type, confidence, content_snippet, source, matched_phrases)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.profile_id,
                event.flag_type,
                event.confidence,
                event.content_snippet,
                event.source,
                matched_phrases_json,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get a flagged event by ID.
    pub fn get_by_id(conn: &Connection, id: i64) -> Result<Option<FlaggedEvent>> {
        let mut stmt = conn.prepare(
            "SELECT f.id, f.profile_id, p.name as profile_name, f.flag_type, f.confidence,
                    f.content_snippet, f.source, f.matched_phrases, f.acknowledged,
                    f.acknowledged_at, f.created_at
             FROM flagged_events f
             LEFT JOIN profiles p ON f.profile_id = p.id
             WHERE f.id = ?1",
        )?;

        let event = stmt
            .query_row([id], |row| {
                Ok(FlaggedEvent {
                    id: row.get(0)?,
                    profile_id: row.get(1)?,
                    profile_name: row.get(2)?,
                    flag_type: row.get(3)?,
                    confidence: row.get(4)?,
                    content_snippet: row.get(5)?,
                    source: row.get(6)?,
                    matched_phrases: parse_json_array(&row.get::<_, String>(7)?),
                    acknowledged: row.get::<_, i32>(8)? != 0,
                    acknowledged_at: row.get::<_, Option<String>>(9)?.map(|s| parse_datetime(&s)),
                    created_at: parse_datetime(&row.get::<_, String>(10)?),
                })
            })
            .ok();

        Ok(event)
    }

    /// Get flagged events with filtering.
    pub fn get_filtered(
        conn: &Connection,
        filter: FlaggedEventFilter,
    ) -> Result<Vec<FlaggedEvent>> {
        let mut sql = String::from(
            "SELECT f.id, f.profile_id, p.name as profile_name, f.flag_type, f.confidence,
                    f.content_snippet, f.source, f.matched_phrases, f.acknowledged,
                    f.acknowledged_at, f.created_at
             FROM flagged_events f
             LEFT JOIN profiles p ON f.profile_id = p.id
             WHERE 1=1",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(profile_id) = filter.profile_id {
            sql.push_str(" AND f.profile_id = ?");
            params_vec.push(Box::new(profile_id));
        }

        if let Some(ref flag_type) = filter.flag_type {
            sql.push_str(" AND f.flag_type = ?");
            params_vec.push(Box::new(flag_type.clone()));
        }

        if let Some(acknowledged) = filter.acknowledged {
            sql.push_str(" AND f.acknowledged = ?");
            params_vec.push(Box::new(if acknowledged { 1 } else { 0 }));
        }

        sql.push_str(" ORDER BY f.created_at DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(" LIMIT ?");
            params_vec.push(Box::new(limit));
        }

        if let Some(offset) = filter.offset {
            sql.push_str(" OFFSET ?");
            params_vec.push(Box::new(offset));
        }

        let mut stmt = conn.prepare(&sql)?;

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let events = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(FlaggedEvent {
                    id: row.get(0)?,
                    profile_id: row.get(1)?,
                    profile_name: row.get(2)?,
                    flag_type: row.get(3)?,
                    confidence: row.get(4)?,
                    content_snippet: row.get(5)?,
                    source: row.get(6)?,
                    matched_phrases: parse_json_array(&row.get::<_, String>(7)?),
                    acknowledged: row.get::<_, i32>(8)? != 0,
                    acknowledged_at: row.get::<_, Option<String>>(9)?.map(|s| parse_datetime(&s)),
                    created_at: parse_datetime(&row.get::<_, String>(10)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(events)
    }

    /// Get recent flagged events with pagination.
    pub fn get_recent(conn: &Connection, limit: i64, offset: i64) -> Result<Vec<FlaggedEvent>> {
        Self::get_filtered(
            conn,
            FlaggedEventFilter {
                limit: Some(limit),
                offset: Some(offset),
                ..Default::default()
            },
        )
    }

    /// Get unacknowledged flagged events.
    pub fn get_unacknowledged(conn: &Connection, limit: i64) -> Result<Vec<FlaggedEvent>> {
        Self::get_filtered(
            conn,
            FlaggedEventFilter {
                acknowledged: Some(false),
                limit: Some(limit),
                ..Default::default()
            },
        )
    }

    /// Acknowledge a flagged event.
    pub fn acknowledge(conn: &Connection, id: i64) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let updated = conn.execute(
            "UPDATE flagged_events SET acknowledged = 1, acknowledged_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(updated > 0)
    }

    /// Acknowledge multiple flagged events.
    pub fn acknowledge_many(conn: &Connection, ids: &[i64]) -> Result<i64> {
        if ids.is_empty() {
            return Ok(0);
        }

        let now = Utc::now().to_rfc3339();
        let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
        let sql = format!(
            "UPDATE flagged_events SET acknowledged = 1, acknowledged_at = ? WHERE id IN ({})",
            placeholders.join(",")
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now)];
        for id in ids {
            params_vec.push(Box::new(*id));
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let updated = conn.execute(&sql, params_refs.as_slice())?;

        Ok(updated as i64)
    }

    /// Delete a flagged event.
    pub fn delete(conn: &Connection, id: i64) -> Result<bool> {
        let deleted = conn.execute("DELETE FROM flagged_events WHERE id = ?1", [id])?;
        Ok(deleted > 0)
    }

    /// Delete flagged events older than a given date.
    pub fn delete_older_than(conn: &Connection, before: DateTime<Utc>) -> Result<i64> {
        let deleted = conn.execute(
            "DELETE FROM flagged_events WHERE created_at < ?1",
            [before.to_rfc3339()],
        )?;
        Ok(deleted as i64)
    }

    /// Count total flagged events.
    pub fn count(conn: &Connection) -> Result<i64> {
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM flagged_events", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Count unacknowledged flagged events.
    pub fn count_unacknowledged(conn: &Connection) -> Result<i64> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM flagged_events WHERE acknowledged = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get statistics for flagged events.
    pub fn get_stats(conn: &Connection) -> Result<FlaggedEventStats> {
        let total: i64 =
            conn.query_row("SELECT COUNT(*) FROM flagged_events", [], |row| row.get(0))?;

        let unacknowledged: i64 = conn.query_row(
            "SELECT COUNT(*) FROM flagged_events WHERE acknowledged = 0",
            [],
            |row| row.get(0),
        )?;

        // Get counts by type
        let mut stmt =
            conn.prepare("SELECT flag_type, COUNT(*) FROM flagged_events GROUP BY flag_type")?;

        let mut by_type = FlaggedTypeCounts::default();

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        for (flag_type, count) in rows.flatten() {
            match flag_type.as_str() {
                "distress" => by_type.distress = count,
                "crisis_indicator" => by_type.crisis_indicator = count,
                "bullying" => by_type.bullying = count,
                "negative_sentiment" => by_type.negative_sentiment = count,
                _ => {}
            }
        }

        Ok(FlaggedEventStats {
            total,
            unacknowledged,
            by_type,
        })
    }
}

/// Create a content snippet from text (truncated and cleaned).
pub fn create_snippet(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .filter(|c| !c.is_control())
        .take(SNIPPET_MAX_LEN)
        .collect();

    if text.len() > SNIPPET_MAX_LEN {
        format!("{}...", cleaned)
    } else {
        cleaned
    }
}

/// Parse a JSON array from string.
fn parse_json_array(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
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

        // Create a test profile
        conn.execute(
            "INSERT INTO profiles (name, os_username, time_rules, content_rules) VALUES ('Test Child', 'testuser', '{}', '{}')",
            [],
        ).unwrap();

        conn
    }

    #[test]
    fn test_insert_and_get_flagged_event() {
        let conn = setup_db();

        let event = NewFlaggedEvent {
            profile_id: 1,
            flag_type: "distress".to_string(),
            confidence: 0.85,
            content_snippet: "I feel so alone and nobody cares".to_string(),
            source: Some("chatgpt.com".to_string()),
            matched_phrases: vec!["feel so alone".to_string(), "nobody cares".to_string()],
        };

        let id = FlaggedEventsRepo::insert(&conn, event).unwrap();
        let retrieved = FlaggedEventsRepo::get_by_id(&conn, id).unwrap().unwrap();

        assert_eq!(retrieved.profile_id, 1);
        assert_eq!(retrieved.profile_name, Some("Test Child".to_string()));
        assert_eq!(retrieved.flag_type, "distress");
        assert!((retrieved.confidence - 0.85).abs() < 0.001);
        assert!(!retrieved.acknowledged);
        assert_eq!(retrieved.matched_phrases.len(), 2);
    }

    #[test]
    fn test_acknowledge_flagged_event() {
        let conn = setup_db();

        let event = NewFlaggedEvent {
            profile_id: 1,
            flag_type: "bullying".to_string(),
            confidence: 0.75,
            content_snippet: "They make fun of me".to_string(),
            source: None,
            matched_phrases: vec!["make fun of me".to_string()],
        };

        let id = FlaggedEventsRepo::insert(&conn, event).unwrap();

        // Initially not acknowledged
        let before = FlaggedEventsRepo::get_by_id(&conn, id).unwrap().unwrap();
        assert!(!before.acknowledged);
        assert!(before.acknowledged_at.is_none());

        // Acknowledge it
        let result = FlaggedEventsRepo::acknowledge(&conn, id).unwrap();
        assert!(result);

        // Now it should be acknowledged
        let after = FlaggedEventsRepo::get_by_id(&conn, id).unwrap().unwrap();
        assert!(after.acknowledged);
        assert!(after.acknowledged_at.is_some());
    }

    #[test]
    fn test_get_unacknowledged() {
        let conn = setup_db();

        // Insert some events
        for i in 0..5 {
            let event = NewFlaggedEvent {
                profile_id: 1,
                flag_type: "distress".to_string(),
                confidence: 0.7,
                content_snippet: format!("Test content {}", i),
                source: None,
                matched_phrases: vec![],
            };
            FlaggedEventsRepo::insert(&conn, event).unwrap();
        }

        // Acknowledge some
        FlaggedEventsRepo::acknowledge(&conn, 1).unwrap();
        FlaggedEventsRepo::acknowledge(&conn, 2).unwrap();

        // Check unacknowledged count
        let unack = FlaggedEventsRepo::get_unacknowledged(&conn, 10).unwrap();
        assert_eq!(unack.len(), 3);
    }

    #[test]
    fn test_get_stats() {
        let conn = setup_db();

        // Insert events of different types
        let types = ["distress", "crisis_indicator", "bullying", "distress"];
        for (i, flag_type) in types.iter().enumerate() {
            let event = NewFlaggedEvent {
                profile_id: 1,
                flag_type: flag_type.to_string(),
                confidence: 0.7,
                content_snippet: format!("Test {}", i),
                source: None,
                matched_phrases: vec![],
            };
            FlaggedEventsRepo::insert(&conn, event).unwrap();
        }

        // Acknowledge one
        FlaggedEventsRepo::acknowledge(&conn, 1).unwrap();

        let stats = FlaggedEventsRepo::get_stats(&conn).unwrap();
        assert_eq!(stats.total, 4);
        assert_eq!(stats.unacknowledged, 3);
        assert_eq!(stats.by_type.distress, 2);
        assert_eq!(stats.by_type.crisis_indicator, 1);
        assert_eq!(stats.by_type.bullying, 1);
        assert_eq!(stats.by_type.negative_sentiment, 0);
    }

    #[test]
    fn test_create_snippet() {
        assert_eq!(create_snippet("short"), "short");

        let long_text = "a".repeat(300);
        let snippet = create_snippet(&long_text);
        assert!(snippet.ends_with("..."));
        assert_eq!(snippet.len(), 203); // 200 + "..."
    }

    #[test]
    fn test_filtered_query() {
        let conn = setup_db();

        // Insert events
        for flag_type in &["distress", "bullying", "distress"] {
            let event = NewFlaggedEvent {
                profile_id: 1,
                flag_type: flag_type.to_string(),
                confidence: 0.7,
                content_snippet: "Test".to_string(),
                source: None,
                matched_phrases: vec![],
            };
            FlaggedEventsRepo::insert(&conn, event).unwrap();
        }

        // Filter by type
        let distress_only = FlaggedEventsRepo::get_filtered(
            &conn,
            FlaggedEventFilter {
                flag_type: Some("distress".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(distress_only.len(), 2);
    }
}

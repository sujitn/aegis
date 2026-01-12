//! Daily statistics repository.

use chrono::{NaiveDate, Utc};
use rusqlite::{params, Connection};

use crate::error::Result;
use crate::models::{Action, CategoryCounts, DailyStats};

/// Repository for daily statistics operations.
pub struct StatsRepo;

impl StatsRepo {
    /// Get or create stats for a given date.
    pub fn get_or_create(conn: &Connection, date: NaiveDate) -> Result<DailyStats> {
        let date_str = date.format("%Y-%m-%d").to_string();

        // Try to get existing stats
        if let Some(stats) = Self::get_by_date(conn, date)? {
            return Ok(stats);
        }

        // Create new stats for the date
        conn.execute(
            "INSERT INTO daily_stats (date, total_prompts, blocked_count, allowed_count, flagged_count, category_counts)
             VALUES (?1, 0, 0, 0, 0, '{}')",
            [&date_str],
        )?;

        Ok(DailyStats {
            date,
            total_prompts: 0,
            blocked_count: 0,
            allowed_count: 0,
            flagged_count: 0,
            category_counts: CategoryCounts::default(),
        })
    }

    /// Get stats for a specific date.
    pub fn get_by_date(conn: &Connection, date: NaiveDate) -> Result<Option<DailyStats>> {
        let date_str = date.format("%Y-%m-%d").to_string();

        let mut stmt = conn.prepare(
            "SELECT date, total_prompts, blocked_count, allowed_count, flagged_count, category_counts
             FROM daily_stats WHERE date = ?1",
        )?;

        let stats = stmt
            .query_row([&date_str], |row| {
                let date_str: String = row.get(0)?;
                let category_counts_str: String = row.get(5)?;

                Ok(DailyStats {
                    date: NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                        .unwrap_or_else(|_| Utc::now().date_naive()),
                    total_prompts: row.get(1)?,
                    blocked_count: row.get(2)?,
                    allowed_count: row.get(3)?,
                    flagged_count: row.get(4)?,
                    category_counts: serde_json::from_str(&category_counts_str).unwrap_or_default(),
                })
            })
            .ok();

        Ok(stats)
    }

    /// Get stats for a date range.
    pub fn get_range(
        conn: &Connection,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<DailyStats>> {
        let start_str = start.format("%Y-%m-%d").to_string();
        let end_str = end.format("%Y-%m-%d").to_string();

        let mut stmt = conn.prepare(
            "SELECT date, total_prompts, blocked_count, allowed_count, flagged_count, category_counts
             FROM daily_stats WHERE date >= ?1 AND date <= ?2 ORDER BY date ASC",
        )?;

        let stats = stmt
            .query_map([&start_str, &end_str], |row| {
                let date_str: String = row.get(0)?;
                let category_counts_str: String = row.get(5)?;

                Ok(DailyStats {
                    date: NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                        .unwrap_or_else(|_| Utc::now().date_naive()),
                    total_prompts: row.get(1)?,
                    blocked_count: row.get(2)?,
                    allowed_count: row.get(3)?,
                    flagged_count: row.get(4)?,
                    category_counts: serde_json::from_str(&category_counts_str).unwrap_or_default(),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(stats)
    }

    /// Increment stats for today.
    pub fn increment(
        conn: &Connection,
        action: Action,
        category: Option<aegis_core::classifier::Category>,
    ) -> Result<()> {
        let today = Utc::now().date_naive();
        let date_str = today.format("%Y-%m-%d").to_string();

        // Ensure the row exists
        Self::get_or_create(conn, today)?;

        // Determine which action column to increment
        let action_column = match action {
            Action::Allowed => "allowed_count",
            Action::Blocked => "blocked_count",
            Action::Flagged => "flagged_count",
        };

        // Increment total and action count
        conn.execute(
            &format!(
                "UPDATE daily_stats SET total_prompts = total_prompts + 1, {} = {} + 1 WHERE date = ?1",
                action_column, action_column
            ),
            [&date_str],
        )?;

        // If there's a category, update the category counts
        if let Some(cat) = category {
            let mut stats = Self::get_by_date(conn, today)?.unwrap_or_else(|| DailyStats {
                date: today,
                total_prompts: 0,
                blocked_count: 0,
                allowed_count: 0,
                flagged_count: 0,
                category_counts: CategoryCounts::default(),
            });

            stats.category_counts.increment(cat);

            let counts_json = serde_json::to_string(&stats.category_counts)?;
            conn.execute(
                "UPDATE daily_stats SET category_counts = ?1 WHERE date = ?2",
                params![counts_json, date_str],
            )?;
        }

        Ok(())
    }

    /// Get total stats (sum of all days).
    pub fn get_totals(conn: &Connection) -> Result<DailyStats> {
        let mut stmt = conn.prepare(
            "SELECT
                COALESCE(SUM(total_prompts), 0),
                COALESCE(SUM(blocked_count), 0),
                COALESCE(SUM(allowed_count), 0),
                COALESCE(SUM(flagged_count), 0)
             FROM daily_stats",
        )?;

        let totals = stmt.query_row([], |row| {
            Ok(DailyStats {
                date: Utc::now().date_naive(),
                total_prompts: row.get(0)?,
                blocked_count: row.get(1)?,
                allowed_count: row.get(2)?,
                flagged_count: row.get(3)?,
                category_counts: CategoryCounts::default(), // Would need separate query for this
            })
        })?;

        Ok(totals)
    }

    /// Delete stats older than a given date.
    pub fn delete_older_than(conn: &Connection, before: NaiveDate) -> Result<i64> {
        let date_str = before.format("%Y-%m-%d").to_string();
        let deleted = conn.execute("DELETE FROM daily_stats WHERE date < ?1", [&date_str])?;
        Ok(deleted as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::run_migrations;
    use aegis_core::classifier::Category;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_get_or_create() {
        let conn = setup_db();
        let today = Utc::now().date_naive();

        // First call creates
        let stats = StatsRepo::get_or_create(&conn, today).unwrap();
        assert_eq!(stats.total_prompts, 0);

        // Second call returns existing
        let stats = StatsRepo::get_or_create(&conn, today).unwrap();
        assert_eq!(stats.total_prompts, 0);
    }

    #[test]
    fn test_increment_stats() {
        let conn = setup_db();
        let today = Utc::now().date_naive();

        // Increment blocked
        StatsRepo::increment(&conn, Action::Blocked, Some(Category::Violence)).unwrap();
        StatsRepo::increment(&conn, Action::Blocked, Some(Category::Violence)).unwrap();
        StatsRepo::increment(&conn, Action::Allowed, None).unwrap();

        let stats = StatsRepo::get_by_date(&conn, today).unwrap().unwrap();
        assert_eq!(stats.total_prompts, 3);
        assert_eq!(stats.blocked_count, 2);
        assert_eq!(stats.allowed_count, 1);
        assert_eq!(stats.category_counts.violence, 2);
    }

    #[test]
    fn test_get_range() {
        let conn = setup_db();
        let today = Utc::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);

        StatsRepo::get_or_create(&conn, today).unwrap();
        StatsRepo::get_or_create(&conn, yesterday).unwrap();

        let range = StatsRepo::get_range(&conn, yesterday, today).unwrap();
        assert_eq!(range.len(), 2);
    }

    #[test]
    fn test_get_totals() {
        let conn = setup_db();

        // Increment some stats
        StatsRepo::increment(&conn, Action::Blocked, None).unwrap();
        StatsRepo::increment(&conn, Action::Blocked, None).unwrap();
        StatsRepo::increment(&conn, Action::Allowed, None).unwrap();

        let totals = StatsRepo::get_totals(&conn).unwrap();
        assert_eq!(totals.total_prompts, 3);
        assert_eq!(totals.blocked_count, 2);
        assert_eq!(totals.allowed_count, 1);
    }
}

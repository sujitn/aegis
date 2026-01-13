//! Data models for storage.

use aegis_core::classifier::Category;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Action taken on a prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Prompt was allowed.
    Allowed,
    /// Prompt was blocked.
    Blocked,
    /// Prompt was flagged for review.
    Flagged,
}

impl Action {
    /// Convert to database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Allowed => "allowed",
            Action::Blocked => "blocked",
            Action::Flagged => "flagged",
        }
    }

    /// Parse from database string.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "allowed" => Some(Action::Allowed),
            "blocked" => Some(Action::Blocked),
            "flagged" => Some(Action::Flagged),
            _ => None,
        }
    }
}

/// A logged event (privacy-preserving: stores hash, not full content).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique identifier.
    pub id: i64,
    /// SHA-256 hash of the prompt (for deduplication).
    pub prompt_hash: String,
    /// Short preview of the prompt (first N chars, redacted).
    pub preview: String,
    /// Detected category (if any).
    pub category: Option<Category>,
    /// Confidence score (0.0 to 1.0).
    pub confidence: Option<f32>,
    /// Action taken.
    pub action: Action,
    /// Source application/site.
    pub source: Option<String>,
    /// Timestamp.
    pub created_at: DateTime<Utc>,
}

/// Parameters for creating a new event.
#[derive(Debug, Clone)]
pub struct NewEvent {
    /// SHA-256 hash of the prompt.
    pub prompt_hash: String,
    /// Short preview of the prompt.
    pub preview: String,
    /// Detected category (if any).
    pub category: Option<Category>,
    /// Confidence score.
    pub confidence: Option<f32>,
    /// Action taken.
    pub action: Action,
    /// Source application/site.
    pub source: Option<String>,
}

/// Daily aggregated statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    /// The date for these stats.
    pub date: NaiveDate,
    /// Total prompts processed.
    pub total_prompts: i64,
    /// Number of prompts blocked.
    pub blocked_count: i64,
    /// Number of prompts allowed.
    pub allowed_count: i64,
    /// Number of prompts flagged.
    pub flagged_count: i64,
    /// Breakdown by category (JSON).
    pub category_counts: CategoryCounts,
}

/// Category breakdown for daily stats.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategoryCounts {
    pub violence: i64,
    pub self_harm: i64,
    pub adult: i64,
    pub jailbreak: i64,
    pub hate: i64,
    pub illegal: i64,
}

impl CategoryCounts {
    /// Increment count for a category.
    pub fn increment(&mut self, category: Category) {
        match category {
            Category::Violence => self.violence += 1,
            Category::SelfHarm => self.self_harm += 1,
            Category::Adult => self.adult += 1,
            Category::Jailbreak => self.jailbreak += 1,
            Category::Hate => self.hate += 1,
            Category::Illegal => self.illegal += 1,
        }
    }
}

/// A filtering rule stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Unique identifier.
    pub id: i64,
    /// Rule name.
    pub name: String,
    /// Whether the rule is enabled.
    pub enabled: bool,
    /// Rule configuration (JSON).
    pub config: serde_json::Value,
    /// Rule priority (lower = higher priority).
    pub priority: i32,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
    /// Updated timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Parameters for creating a new rule.
#[derive(Debug, Clone)]
pub struct NewRule {
    /// Rule name.
    pub name: String,
    /// Whether the rule is enabled.
    pub enabled: bool,
    /// Rule configuration (JSON).
    pub config: serde_json::Value,
    /// Rule priority.
    pub priority: i32,
}

/// Application configuration stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Configuration key.
    pub key: String,
    /// Configuration value (JSON).
    pub value: serde_json::Value,
}

/// Authentication data.
#[derive(Debug, Clone)]
pub struct Auth {
    /// Unique identifier (always 1 for single-user).
    pub id: i64,
    /// Argon2 password hash.
    pub password_hash: String,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
    /// Last login timestamp.
    pub last_login: Option<DateTime<Utc>>,
}

/// A user profile stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Unique identifier.
    pub id: i64,
    /// Profile display name.
    pub name: String,
    /// OS username to match (optional, None = manual selection only).
    pub os_username: Option<String>,
    /// Time rules configuration (JSON).
    pub time_rules: serde_json::Value,
    /// Content rules configuration (JSON).
    pub content_rules: serde_json::Value,
    /// Whether this profile is enabled.
    pub enabled: bool,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
    /// Updated timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Parameters for creating a new profile.
#[derive(Debug, Clone)]
pub struct NewProfile {
    /// Profile display name.
    pub name: String,
    /// OS username to match (optional).
    pub os_username: Option<String>,
    /// Time rules configuration (JSON).
    pub time_rules: serde_json::Value,
    /// Content rules configuration (JSON).
    pub content_rules: serde_json::Value,
    /// Whether this profile is enabled.
    pub enabled: bool,
}

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
    pub profanity: i64,
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
            Category::Profanity => self.profanity += 1,
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

/// Configuration for sentiment analysis on a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSentimentConfig {
    /// Whether sentiment analysis is enabled for this profile.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Sensitivity threshold (0.0 to 1.0, lower = more sensitive).
    #[serde(default = "default_sensitivity")]
    pub sensitivity: f32,
    /// Whether to detect distress signals.
    #[serde(default = "default_true")]
    pub detect_distress: bool,
    /// Whether to detect crisis indicators.
    #[serde(default = "default_true")]
    pub detect_crisis: bool,
    /// Whether to detect bullying.
    #[serde(default = "default_true")]
    pub detect_bullying: bool,
    /// Whether to detect negative sentiment.
    #[serde(default = "default_true")]
    pub detect_negative: bool,
}

fn default_true() -> bool {
    true
}

fn default_sensitivity() -> f32 {
    0.5
}

impl Default for ProfileSentimentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sensitivity: 0.5,
            detect_distress: true,
            detect_crisis: true,
            detect_bullying: true,
            detect_negative: true,
        }
    }
}

/// NSFW threshold preset for image filtering.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NsfwThresholdPreset {
    /// Child profile (< 13): Very aggressive blocking (0.3).
    Child,
    /// Teen profile (13-17): Balanced blocking (0.5).
    #[default]
    Teen,
    /// Adult profile (18+): Permissive blocking (0.8).
    Adult,
    /// Custom threshold value.
    Custom(f32),
}

impl NsfwThresholdPreset {
    /// Returns the threshold value for this preset.
    pub fn threshold(&self) -> f32 {
        match self {
            NsfwThresholdPreset::Child => 0.3,
            NsfwThresholdPreset::Teen => 0.5,
            NsfwThresholdPreset::Adult => 0.8,
            NsfwThresholdPreset::Custom(t) => t.clamp(0.0, 1.0),
        }
    }
}

/// Configuration for image filtering on a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileImageFilteringConfig {
    /// Whether image filtering is enabled for this profile.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// NSFW threshold preset (child/teen/adult/custom).
    #[serde(default)]
    pub nsfw_threshold: NsfwThresholdPreset,
}

impl Default for ProfileImageFilteringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            nsfw_threshold: NsfwThresholdPreset::Teen,
        }
    }
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
    /// Sentiment analysis configuration.
    pub sentiment_config: ProfileSentimentConfig,
    /// Image filtering configuration (F033).
    pub image_filtering_config: ProfileImageFilteringConfig,
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
    /// Sentiment analysis configuration.
    pub sentiment_config: ProfileSentimentConfig,
    /// Image filtering configuration (F033).
    pub image_filtering_config: ProfileImageFilteringConfig,
}

/// A site entry stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    /// Unique identifier.
    pub id: i64,
    /// Domain pattern (exact or wildcard).
    pub pattern: String,
    /// Human-friendly display name.
    pub name: String,
    /// Site category (consumer, api, enterprise).
    pub category: String,
    /// Parser ID for F026 integration.
    pub parser_id: Option<String>,
    /// Whether this site is enabled.
    pub enabled: bool,
    /// Source of this entry (bundled, remote, custom).
    pub source: String,
    /// Priority for pattern matching.
    pub priority: i32,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
    /// Updated timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Parameters for creating a new site.
#[derive(Debug, Clone)]
pub struct NewSite {
    /// Domain pattern (exact or wildcard).
    pub pattern: String,
    /// Human-friendly display name.
    pub name: String,
    /// Site category (consumer, api, enterprise).
    pub category: String,
    /// Parser ID for F026 integration.
    pub parser_id: Option<String>,
    /// Whether this site is enabled.
    pub enabled: bool,
    /// Source of this entry (bundled, remote, custom).
    pub source: String,
    /// Priority for pattern matching.
    pub priority: i32,
}

/// A disabled bundled site pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisabledBundledSite {
    /// The pattern that is disabled.
    pub pattern: String,
    /// When it was disabled.
    pub disabled_at: DateTime<Utc>,
}

/// A flagged event for parental review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlaggedEvent {
    /// Unique identifier.
    pub id: i64,
    /// Profile ID that triggered this flag.
    pub profile_id: i64,
    /// Profile name (for display).
    pub profile_name: Option<String>,
    /// Sentiment flag type.
    pub flag_type: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// Short preview of the content (privacy-preserving).
    pub content_snippet: String,
    /// Source application/site.
    pub source: Option<String>,
    /// Matched phrases that triggered the flag.
    pub matched_phrases: Vec<String>,
    /// Whether this has been acknowledged by parent.
    pub acknowledged: bool,
    /// When it was acknowledged.
    pub acknowledged_at: Option<DateTime<Utc>>,
    /// Timestamp.
    pub created_at: DateTime<Utc>,
}

/// Parameters for creating a new flagged event.
#[derive(Debug, Clone)]
pub struct NewFlaggedEvent {
    /// Profile ID that triggered this flag.
    pub profile_id: i64,
    /// Sentiment flag type (distress, crisis_indicator, bullying, negative_sentiment).
    pub flag_type: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// Short preview of the content.
    pub content_snippet: String,
    /// Source application/site.
    pub source: Option<String>,
    /// Matched phrases that triggered the flag.
    pub matched_phrases: Vec<String>,
}

/// Filter options for querying flagged events.
#[derive(Debug, Clone, Default)]
pub struct FlaggedEventFilter {
    /// Filter by profile ID.
    pub profile_id: Option<i64>,
    /// Filter by flag type.
    pub flag_type: Option<String>,
    /// Filter by acknowledgment status.
    pub acknowledged: Option<bool>,
    /// Maximum number of results.
    pub limit: Option<i64>,
    /// Offset for pagination.
    pub offset: Option<i64>,
}

/// Summary statistics for flagged events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlaggedEventStats {
    /// Total flagged events.
    pub total: i64,
    /// Unacknowledged flagged events.
    pub unacknowledged: i64,
    /// Breakdown by flag type.
    pub by_type: FlaggedTypeCounts,
}

/// Breakdown of flagged events by type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlaggedTypeCounts {
    pub distress: i64,
    pub crisis_indicator: i64,
    pub bullying: i64,
    pub negative_sentiment: i64,
}

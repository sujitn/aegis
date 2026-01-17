//! API request and response models.

use aegis_core::classifier::{Category, ClassificationTier};
use aegis_core::rule_engine::RuleAction;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Request body for POST /api/check.
#[derive(Debug, Deserialize)]
pub struct CheckRequest {
    /// The prompt text to classify.
    pub prompt: String,
    /// Optional OS username for profile lookup.
    pub os_username: Option<String>,
}

/// Category match in the response.
#[derive(Debug, Serialize)]
pub struct CategoryMatchResponse {
    pub category: Category,
    pub confidence: f32,
    pub tier: ClassificationTier,
}

/// Response body for POST /api/check.
#[derive(Debug, Serialize)]
pub struct CheckResponse {
    /// Action to take: allow, warn, or block.
    pub action: RuleAction,
    /// Reason for the action (rule name or "allowed").
    pub reason: String,
    /// Matched categories.
    pub categories: Vec<CategoryMatchResponse>,
    /// Classification latency in milliseconds.
    pub latency_ms: u64,
}

/// Request body for POST /api/auth/verify.
#[derive(Debug, Deserialize)]
pub struct AuthVerifyRequest {
    /// Parent password.
    pub password: String,
}

/// Response body for POST /api/auth/verify.
#[derive(Debug, Serialize)]
pub struct AuthVerifyResponse {
    /// Whether authentication was successful.
    pub success: bool,
    /// Session token (only present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
}

/// Query parameters for GET /api/logs.
#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    /// Maximum number of logs to return (default: 50).
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Offset for pagination (default: 0).
    #[serde(default)]
    pub offset: i64,
    /// Filter by action (optional).
    pub action: Option<String>,
}

fn default_limit() -> i64 {
    50
}

/// Log entry in the response.
#[derive(Debug, Serialize)]
pub struct LogEntry {
    pub id: i64,
    pub preview: String,
    pub category: Option<Category>,
    pub confidence: Option<f32>,
    pub action: String,
    pub source: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Response body for GET /api/logs.
#[derive(Debug, Serialize)]
pub struct LogsResponse {
    pub logs: Vec<LogEntry>,
    pub total: i64,
}

/// Response body for GET /api/stats.
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_prompts: i64,
    pub blocked_count: i64,
    pub warned_count: i64,
    pub allowed_count: i64,
    pub category_counts: CategoryCountsResponse,
}

/// Category counts in stats response.
#[derive(Debug, Serialize)]
pub struct CategoryCountsResponse {
    pub violence: i64,
    pub self_harm: i64,
    pub adult: i64,
    pub jailbreak: i64,
    pub hate: i64,
    pub illegal: i64,
}

/// Rule entry in the response.
#[derive(Debug, Serialize)]
pub struct RuleEntry {
    pub id: i64,
    pub name: String,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response body for GET /api/rules.
#[derive(Debug, Serialize)]
pub struct RulesResponse {
    pub rules: Vec<RuleEntry>,
}

/// Request body for PUT /api/rules.
#[derive(Debug, Deserialize)]
pub struct UpdateRulesRequest {
    /// Session token for authentication.
    pub session_token: String,
    /// Rules to update.
    pub rules: Vec<RuleUpdate>,
}

/// A single rule update.
#[derive(Debug, Deserialize)]
pub struct RuleUpdate {
    pub id: i64,
    pub name: String,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub priority: i32,
}

/// Response body for PUT /api/rules.
#[derive(Debug, Serialize)]
pub struct UpdateRulesResponse {
    pub success: bool,
    pub updated_count: usize,
}

// ===== Flagged Events API =====

/// Query parameters for GET /api/flagged.
#[derive(Debug, Deserialize)]
pub struct FlaggedQuery {
    /// Maximum number of items to return (default: 50).
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Offset for pagination (default: 0).
    #[serde(default)]
    pub offset: i64,
    /// Filter by flag type (optional).
    pub flag_type: Option<String>,
    /// Include acknowledged items (default: false).
    #[serde(default)]
    pub include_acknowledged: bool,
}

/// Flagged event entry in the response.
#[derive(Debug, Serialize)]
pub struct FlaggedEntry {
    pub id: i64,
    pub profile_id: i64,
    pub profile_name: Option<String>,
    pub flag_type: String,
    pub confidence: f32,
    pub content_snippet: String,
    pub source: Option<String>,
    pub matched_phrases: Vec<String>,
    pub acknowledged: bool,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Response body for GET /api/flagged.
#[derive(Debug, Serialize)]
pub struct FlaggedResponse {
    pub items: Vec<FlaggedEntry>,
    pub total: i64,
    pub unacknowledged: i64,
}

/// Response body for GET /api/flagged/stats.
#[derive(Debug, Serialize)]
pub struct FlaggedStatsResponse {
    pub total: i64,
    pub unacknowledged: i64,
    pub by_type: FlaggedTypeCounts,
}

/// Counts by flag type.
#[derive(Debug, Serialize)]
pub struct FlaggedTypeCounts {
    pub distress: i64,
    pub crisis_indicator: i64,
    pub bullying: i64,
    pub negative_sentiment: i64,
}

/// Request body for POST /api/flagged/:id/acknowledge.
#[derive(Debug, Deserialize)]
pub struct AcknowledgeRequest {
    /// Session token for authentication.
    pub session_token: String,
}

/// Request body for POST /api/flagged/acknowledge-all.
#[derive(Debug, Deserialize)]
pub struct AcknowledgeAllRequest {
    /// Session token for authentication.
    pub session_token: String,
    /// Optional list of IDs to acknowledge (if empty, acknowledges all).
    #[serde(default)]
    pub ids: Vec<i64>,
}

/// Response body for acknowledge operations.
#[derive(Debug, Serialize)]
pub struct AcknowledgeResponse {
    pub success: bool,
    pub acknowledged_count: usize,
}

/// Request body for DELETE /api/flagged/:id.
#[derive(Debug, Deserialize)]
pub struct DeleteFlaggedRequest {
    /// Session token for authentication.
    pub session_token: String,
}

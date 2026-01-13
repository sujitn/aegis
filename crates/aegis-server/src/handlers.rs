//! API route handlers.

use axum::extract::{Query, State};
use axum::Json;
use tracing::{debug, info};

use aegis_core::auth::SessionToken;
use aegis_storage::{models::Action, NewRule};

use crate::error::{ApiError, Result};
use crate::models::{
    AuthVerifyRequest, AuthVerifyResponse, CategoryCountsResponse, CategoryMatchResponse,
    CheckRequest, CheckResponse, LogEntry, LogsQuery, LogsResponse, RuleEntry, RulesResponse,
    StatsResponse, UpdateRulesRequest, UpdateRulesResponse,
};
use crate::state::AppState;

/// POST /api/check - Classify a prompt and return action.
pub async fn check_prompt(
    State(state): State<AppState>,
    Json(req): Json<CheckRequest>,
) -> Result<Json<CheckResponse>> {
    debug!(
        prompt_len = req.prompt.len(),
        os_username = ?req.os_username,
        "Checking prompt"
    );

    // Classify the prompt
    let classification = {
        let mut classifier = state.classifier.write().unwrap();
        classifier.classify(&req.prompt)
    };

    // Get the rule engine (use profile-specific rules if os_username provided)
    let rule_result = {
        let profiles = state.profiles.read().unwrap();
        let rules = state.rules.read().unwrap();

        // If os_username provided and matches a profile, use that profile's rules
        if let Some(ref username) = req.os_username {
            if let Some(profile) = profiles.get_by_os_username(username) {
                // Create a temporary rule engine with profile's rules
                let profile_engine = aegis_core::rule_engine::RuleEngine {
                    time_rules: profile.time_rules.clone(),
                    content_rules: profile.content_rules.clone(),
                };
                profile_engine.evaluate_now(&classification)
            } else {
                // No profile found - use default rules
                rules.evaluate_now(&classification)
            }
        } else {
            rules.evaluate_now(&classification)
        }
    };

    // Log the event
    let category = classification.highest_confidence().map(|m| m.category);
    let confidence = classification.highest_confidence().map(|m| m.confidence);
    let action = match rule_result.action {
        aegis_core::rule_engine::RuleAction::Allow => Action::Allowed,
        aegis_core::rule_engine::RuleAction::Warn => Action::Flagged,
        aegis_core::rule_engine::RuleAction::Block => Action::Blocked,
    };

    let _ = state.db.log_event(
        &req.prompt,
        category,
        confidence,
        action,
        Some("api".to_string()),
    );

    // Build response
    let reason = if rule_result.source.has_rule() {
        rule_result.source.rule_name().unwrap_or("rule").to_string()
    } else {
        "allowed".to_string()
    };

    let categories: Vec<CategoryMatchResponse> = classification
        .matches
        .iter()
        .map(|m| CategoryMatchResponse {
            category: m.category,
            confidence: m.confidence,
            tier: m.tier,
        })
        .collect();

    let latency_ms = classification.duration_us / 1000;

    info!(
        action = ?rule_result.action,
        latency_ms,
        categories = categories.len(),
        "Prompt check complete"
    );

    Ok(Json(CheckResponse {
        action: rule_result.action,
        reason,
        categories,
        latency_ms,
    }))
}

/// GET /api/stats - Get aggregated statistics.
pub async fn get_stats(State(state): State<AppState>) -> Result<Json<StatsResponse>> {
    let stats = state.db.get_total_stats()?;

    Ok(Json(StatsResponse {
        total_prompts: stats.total_prompts,
        blocked_count: stats.blocked_count,
        warned_count: stats.flagged_count,
        allowed_count: stats.allowed_count,
        category_counts: CategoryCountsResponse {
            violence: stats.category_counts.violence,
            self_harm: stats.category_counts.self_harm,
            adult: stats.category_counts.adult,
            jailbreak: stats.category_counts.jailbreak,
            hate: stats.category_counts.hate,
            illegal: stats.category_counts.illegal,
        },
    }))
}

/// GET /api/logs - Get event logs with pagination.
pub async fn get_logs(
    State(state): State<AppState>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<LogsResponse>> {
    let events = if let Some(ref action_str) = query.action {
        let action = parse_action(action_str)?;
        state
            .db
            .get_events_by_action(action, query.limit, query.offset)?
    } else {
        state.db.get_recent_events(query.limit, query.offset)?
    };

    let total = state.db.count_events()?;

    let logs: Vec<LogEntry> = events
        .into_iter()
        .map(|e| LogEntry {
            id: e.id,
            preview: e.preview,
            category: e.category,
            confidence: e.confidence,
            action: format!("{:?}", e.action).to_lowercase(),
            source: e.source,
            created_at: e.created_at,
        })
        .collect();

    Ok(Json(LogsResponse { logs, total }))
}

/// GET /api/rules - Get all rules.
pub async fn get_rules(State(state): State<AppState>) -> Result<Json<RulesResponse>> {
    let rules = state.db.get_all_rules()?;

    let rule_entries: Vec<RuleEntry> = rules
        .into_iter()
        .map(|r| RuleEntry {
            id: r.id,
            name: r.name,
            enabled: r.enabled,
            config: r.config,
            priority: r.priority,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect();

    Ok(Json(RulesResponse {
        rules: rule_entries,
    }))
}

/// PUT /api/rules - Update rules (requires auth).
pub async fn update_rules(
    State(state): State<AppState>,
    Json(req): Json<UpdateRulesRequest>,
) -> Result<Json<UpdateRulesResponse>> {
    // Validate session token
    let token = SessionToken::from_string(req.session_token);
    if !state.auth.validate_session(&token) {
        return Err(ApiError::SessionExpired);
    }

    // Update rules
    let mut updated_count = 0;
    for rule_update in req.rules {
        let new_rule = NewRule {
            name: rule_update.name,
            enabled: rule_update.enabled,
            config: rule_update.config,
            priority: rule_update.priority,
        };

        if state.db.update_rule(rule_update.id, new_rule).is_ok() {
            updated_count += 1;
        }
    }

    info!(updated_count, "Rules updated");

    Ok(Json(UpdateRulesResponse {
        success: true,
        updated_count,
    }))
}

/// POST /api/auth/verify - Verify password and get session token.
pub async fn verify_auth(
    State(state): State<AppState>,
    Json(req): Json<AuthVerifyRequest>,
) -> Result<Json<AuthVerifyResponse>> {
    // Check if auth is set up
    if !state.db.is_auth_setup()? {
        return Err(ApiError::BadRequest(
            "password not set - setup required".to_string(),
        ));
    }

    // Get stored hash
    let hash = state.db.get_password_hash()?;

    // Verify password
    let is_valid = state
        .auth
        .verify_password(&req.password, &hash)
        .map_err(|_| ApiError::InvalidCredentials)?;

    if !is_valid {
        return Ok(Json(AuthVerifyResponse {
            success: false,
            session_token: None,
        }));
    }

    // Create session
    let token = state.auth.create_session();

    // Update last login
    let _ = state.db.update_last_login();

    info!("Authentication successful, session created");

    Ok(Json(AuthVerifyResponse {
        success: true,
        session_token: Some(token.as_str().to_string()),
    }))
}

/// Parse action string to Action enum.
fn parse_action(s: &str) -> Result<Action> {
    match s.to_lowercase().as_str() {
        "allowed" => Ok(Action::Allowed),
        "blocked" => Ok(Action::Blocked),
        "flagged" | "warned" => Ok(Action::Flagged),
        _ => Err(ApiError::BadRequest(format!("invalid action: {}", s))),
    }
}

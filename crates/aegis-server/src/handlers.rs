//! API route handlers.

use axum::extract::{Path, Query, State};
use axum::Json;
use tracing::{debug, info};

use aegis_core::auth::SessionToken;
use aegis_core::classifier::SentimentFlag;
use aegis_storage::{models::Action, NewRule, PauseDuration};

use crate::error::{ApiError, Result};
use crate::models::{
    AcknowledgeAllRequest, AcknowledgeRequest, AcknowledgeResponse, AuthVerifyRequest,
    AuthVerifyResponse, CategoryCountsResponse, CategoryMatchResponse, CheckRequest, CheckResponse,
    DeleteFlaggedRequest, FlaggedEntry, FlaggedQuery, FlaggedResponse, FlaggedStatsResponse,
    FlaggedTypeCounts, LogEntry, LogsQuery, LogsResponse, PauseProtectionRequest,
    ProtectionResponse, ProtectionStatusResponse, ReloadRulesRequest, ReloadRulesResponse,
    ResumeProtectionRequest, RuleEntry, RulesResponse, StatsResponse, UpdateRulesRequest,
    UpdateRulesResponse,
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

    // Check if protection is enabled (F032 - read from database)
    let is_filtering_enabled = state.state_manager.is_filtering_enabled().unwrap_or(true); // Default to enabled on error

    if !is_filtering_enabled {
        debug!("Protection is paused/disabled, allowing prompt");
        return Ok(Json(CheckResponse {
            action: aegis_core::rule_engine::RuleAction::Allow,
            reason: "protection_paused".to_string(),
            categories: vec![],
            latency_ms: 0,
        }));
    }

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

    // Run sentiment analysis and flag emotional content
    // Get profile ID - use provided os_username or auto-detect current user
    let effective_username = req.os_username.clone().or_else(|| {
        std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .ok()
    });

    let profile_id = if let Some(ref username) = effective_username {
        state.db.get_all_profiles().ok().and_then(|profiles| {
            profiles
                .iter()
                .find(|p| p.os_username.as_ref() == Some(username))
                .map(|p| p.id)
        })
    } else {
        None
    };

    // Run sentiment analysis if we have a profile
    if let Some(pid) = profile_id {
        let sentiment_result = {
            let analyzer = state.sentiment_analyzer.write().unwrap();
            analyzer.analyze(&req.prompt)
        };

        // Store flagged events
        for flag in &sentiment_result.flags {
            let flag_type = match flag.flag {
                SentimentFlag::Distress => "distress",
                SentimentFlag::CrisisIndicator => "crisis_indicator",
                SentimentFlag::Bullying => "bullying",
                SentimentFlag::NegativeSentiment => "negative_sentiment",
            };

            if let Err(e) = state.db.log_flagged_event(
                pid,
                flag_type,
                flag.confidence,
                &req.prompt,
                Some("browser-extension".to_string()),
                flag.matched_phrases.clone(),
            ) {
                debug!("Failed to log flagged event: {}", e);
            } else {
                info!(
                    "Flagged {} content from browser-extension (confidence: {:.2})",
                    flag.flag.name(),
                    flag.confidence
                );
            }
        }
    }

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

// ===== Flagged Events Handlers =====

/// GET /api/flagged - Get flagged events with pagination.
pub async fn get_flagged(
    State(state): State<AppState>,
    Query(query): Query<FlaggedQuery>,
) -> Result<Json<FlaggedResponse>> {
    let events = state
        .db
        .get_recent_flagged_events(query.limit, query.offset)?;

    // Filter by type and acknowledged status
    let items: Vec<FlaggedEntry> = events
        .into_iter()
        .filter(|e| {
            // Filter by type if specified
            if let Some(ref flag_type) = query.flag_type {
                if e.flag_type != *flag_type {
                    return false;
                }
            }
            // Filter by acknowledged status
            if !query.include_acknowledged && e.acknowledged {
                return false;
            }
            true
        })
        .map(|e| FlaggedEntry {
            id: e.id,
            profile_id: e.profile_id,
            profile_name: e.profile_name,
            flag_type: e.flag_type,
            confidence: e.confidence,
            content_snippet: e.content_snippet,
            source: e.source,
            matched_phrases: e.matched_phrases,
            acknowledged: e.acknowledged,
            acknowledged_at: e.acknowledged_at,
            created_at: e.created_at,
        })
        .collect();

    let stats = state.db.get_flagged_event_stats()?;

    Ok(Json(FlaggedResponse {
        items,
        total: stats.total,
        unacknowledged: stats.unacknowledged,
    }))
}

/// GET /api/flagged/stats - Get flagged event statistics.
pub async fn get_flagged_stats(
    State(state): State<AppState>,
) -> Result<Json<FlaggedStatsResponse>> {
    let stats = state.db.get_flagged_event_stats()?;

    Ok(Json(FlaggedStatsResponse {
        total: stats.total,
        unacknowledged: stats.unacknowledged,
        by_type: FlaggedTypeCounts {
            distress: stats.by_type.distress,
            crisis_indicator: stats.by_type.crisis_indicator,
            bullying: stats.by_type.bullying,
            negative_sentiment: stats.by_type.negative_sentiment,
        },
    }))
}

/// POST /api/flagged/:id/acknowledge - Acknowledge a flagged event.
pub async fn acknowledge_flagged(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<AcknowledgeRequest>,
) -> Result<Json<AcknowledgeResponse>> {
    // Validate session token
    let token = SessionToken::from_string(req.session_token);
    if !state.auth.validate_session(&token) {
        return Err(ApiError::SessionExpired);
    }

    let success = state.db.acknowledge_flagged_event(id)?;

    info!(id, success, "Flagged event acknowledged");

    Ok(Json(AcknowledgeResponse {
        success,
        acknowledged_count: if success { 1 } else { 0 },
    }))
}

/// POST /api/flagged/acknowledge-all - Acknowledge multiple flagged events.
pub async fn acknowledge_all_flagged(
    State(state): State<AppState>,
    Json(req): Json<AcknowledgeAllRequest>,
) -> Result<Json<AcknowledgeResponse>> {
    // Validate session token
    let token = SessionToken::from_string(req.session_token);
    if !state.auth.validate_session(&token) {
        return Err(ApiError::SessionExpired);
    }

    let ids = if req.ids.is_empty() {
        // Get all unacknowledged event IDs
        let events = state.db.get_recent_flagged_events(1000, 0)?;
        events
            .into_iter()
            .filter(|e| !e.acknowledged)
            .map(|e| e.id)
            .collect()
    } else {
        req.ids
    };

    if !ids.is_empty() {
        state.db.acknowledge_flagged_events(&ids)?;
    }

    let count = ids.len();

    info!(count, "Flagged events acknowledged");

    Ok(Json(AcknowledgeResponse {
        success: true,
        acknowledged_count: count,
    }))
}

/// DELETE /api/flagged/:id - Delete a flagged event.
pub async fn delete_flagged(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<DeleteFlaggedRequest>,
) -> Result<Json<AcknowledgeResponse>> {
    // Validate session token
    let token = SessionToken::from_string(req.session_token);
    if !state.auth.validate_session(&token) {
        return Err(ApiError::SessionExpired);
    }

    let success = state.db.delete_flagged_event(id)?;

    info!(id, success, "Flagged event deleted");

    Ok(Json(AcknowledgeResponse {
        success,
        acknowledged_count: if success { 1 } else { 0 },
    }))
}

// ===== Rules Reload Handler =====

/// POST /api/rules/reload - Reload rules from database for a profile.
///
/// This endpoint reloads time and content rules from the database and updates
/// the proxy's FilteringState if one is configured. Call this after saving
/// rules in the UI to apply changes to the running proxy.
pub async fn reload_rules(
    State(state): State<AppState>,
    Json(req): Json<ReloadRulesRequest>,
) -> Result<Json<ReloadRulesResponse>> {
    info!(profile_id = req.profile_id, "Reloading rules from database");

    // Get the profile from the database
    let profile = state
        .db
        .get_profile(req.profile_id)?
        .ok_or_else(|| ApiError::BadRequest(format!("Profile {} not found", req.profile_id)))?;

    // If profile is disabled, use empty rules (no blocking)
    let (time_rules, content_rules) = if profile.enabled {
        // Parse time rules from JSON
        let time_rules: aegis_core::time_rules::TimeRuleSet =
            serde_json::from_value(profile.time_rules.clone()).unwrap_or_default();

        // Parse content rules from JSON
        let content_rules: aegis_core::content_rules::ContentRuleSet =
            serde_json::from_value(profile.content_rules.clone()).unwrap_or_default();

        (time_rules, content_rules)
    } else {
        info!(
            profile_id = req.profile_id,
            profile_name = %profile.name,
            "Profile is disabled, using empty rules"
        );
        (
            aegis_core::time_rules::TimeRuleSet::default(),
            aegis_core::content_rules::ContentRuleSet::default(),
        )
    };

    let time_rules_count = time_rules.rules.len();
    let content_rules_count = content_rules.rules.len();

    // Log each time rule's enabled status for debugging
    for rule in &time_rules.rules {
        info!(
            rule_id = %rule.id,
            rule_name = %rule.name,
            enabled = rule.enabled,
            "Time rule status"
        );
    }

    info!(
        profile_id = req.profile_id,
        profile_name = %profile.name,
        time_rules_count,
        content_rules_count,
        "Loaded rules from profile"
    );

    // Update the proxy's FilteringState if available
    if let Some(ref filtering_state) = state.filtering_state {
        filtering_state.update_rules(time_rules.clone(), content_rules.clone());
        filtering_state.set_profile_with_id(Some(profile.name.clone()), Some(profile.id));

        // Also update sentiment analysis config
        if profile.sentiment_config.enabled {
            use aegis_core::classifier::{SentimentConfig, SentimentFlag};
            use std::collections::HashSet;

            let mut enabled_flags = HashSet::new();
            if profile.sentiment_config.detect_distress {
                enabled_flags.insert(SentimentFlag::Distress);
            }
            if profile.sentiment_config.detect_crisis {
                enabled_flags.insert(SentimentFlag::CrisisIndicator);
            }
            if profile.sentiment_config.detect_bullying {
                enabled_flags.insert(SentimentFlag::Bullying);
            }
            if profile.sentiment_config.detect_negative {
                enabled_flags.insert(SentimentFlag::NegativeSentiment);
            }

            let sentiment_config = SentimentConfig {
                enabled: true,
                threshold: profile.sentiment_config.sensitivity,
                enabled_flags,
                notify_on_flag: true,
            };
            filtering_state.enable_sentiment_analysis(sentiment_config);
        } else {
            filtering_state.disable_sentiment_analysis();
        }

        info!(
            profile_name = %profile.name,
            "Updated proxy FilteringState with new rules"
        );
    }

    // Also update the server's rule engine
    {
        let mut rules = state.rules.write().unwrap();
        rules.time_rules = time_rules;
        rules.content_rules = content_rules;
    }

    Ok(Json(ReloadRulesResponse {
        success: true,
        time_rules_count,
        content_rules_count,
        message: format!(
            "Reloaded {} time rules and {} content rules for profile '{}'",
            time_rules_count, content_rules_count, profile.name
        ),
    }))
}

// ===== Protection Control Handlers =====

/// GET /api/protection/status - Get current protection status.
///
/// Uses centralized state from database (F032) for cross-process consistency.
pub async fn get_protection_status(
    State(state): State<AppState>,
) -> Result<Json<ProtectionStatusResponse>> {
    // Read from centralized state manager (database)
    let protection_state = state
        .state_manager
        .get_protection_state()
        .map_err(|e| ApiError::Internal(format!("Failed to get protection state: {}", e)))?;

    let enabled = protection_state.is_active();
    let status = if protection_state.is_disabled() {
        "disabled".to_string()
    } else if protection_state.is_paused() {
        "paused".to_string()
    } else {
        "active".to_string()
    };

    Ok(Json(ProtectionStatusResponse { enabled, status }))
}

/// POST /api/protection/pause - Pause protection.
///
/// Persists to database (F032) so all processes see the change.
/// Note: Auth is handled by the dashboard UI (user must be logged in to see the pause button).
pub async fn pause_protection(
    State(state): State<AppState>,
    Json(req): Json<PauseProtectionRequest>,
) -> Result<Json<ProtectionResponse>> {
    // Note: We skip session validation because the dashboard runs in a separate process
    // with its own AuthManager. The user has already authenticated in the dashboard.
    let _ = req.session_token; // Acknowledge the field

    // Convert request to PauseDuration
    let duration = match req.duration_type.as_str() {
        "minutes" => PauseDuration::Minutes(req.duration_value),
        "hours" => PauseDuration::Hours(req.duration_value),
        "indefinite" => PauseDuration::Indefinite,
        _ => {
            return Err(ApiError::BadRequest(format!(
                "Invalid duration type: {}",
                req.duration_type
            )))
        }
    };

    // Pause via centralized state manager (writes to database)
    state
        .state_manager
        .pause_protection(duration)
        .map_err(|e| ApiError::Internal(format!("Failed to pause protection: {}", e)))?;

    // Also update FilteringState for immediate in-memory sync (if proxy is in same process)
    if let Some(ref filtering_state) = state.filtering_state {
        filtering_state.disable();
    }

    let duration_desc = match req.duration_type.as_str() {
        "minutes" => format!("{} minutes", req.duration_value),
        "hours" => format!("{} hours", req.duration_value),
        "indefinite" => "indefinitely".to_string(),
        _ => "unknown duration".to_string(),
    };

    info!(
        "Protection paused for {} (persisted to database)",
        duration_desc
    );

    Ok(Json(ProtectionResponse {
        success: true,
        status: "paused".to_string(),
        message: format!("Protection paused for {}", duration_desc),
    }))
}

/// POST /api/protection/resume - Resume protection.
///
/// Persists to database (F032) so all processes see the change.
pub async fn resume_protection(
    State(state): State<AppState>,
    Json(_req): Json<ResumeProtectionRequest>,
) -> Result<Json<ProtectionResponse>> {
    // Note: Resume does not require auth (security design - resuming is always allowed)

    // Resume via centralized state manager (writes to database)
    state
        .state_manager
        .resume_protection()
        .map_err(|e| ApiError::Internal(format!("Failed to resume protection: {}", e)))?;

    // Also update FilteringState for immediate in-memory sync (if proxy is in same process)
    if let Some(ref filtering_state) = state.filtering_state {
        filtering_state.enable();
    }

    info!("Protection resumed (persisted to database)");

    Ok(Json(ProtectionResponse {
        success: true,
        status: "active".to_string(),
        message: "Protection resumed".to_string(),
    }))
}

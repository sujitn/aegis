//! HTTP request handler with classification and rule evaluation.
//!
//! Processes intercepted requests, applies classification, and decides
//! whether to block or forward.
//!
//! Supports profile-aware filtering - when filtering is disabled (e.g., parent
//! profile active), requests pass through without classification.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use http_body_util::{BodyExt, Full};
use hudsucker::{
    hyper::{Request, Response},
    tokio_tungstenite::tungstenite::Message,
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler,
};
use hyper::body::Bytes;
use parking_lot::RwLock;

/// Helper to convert bytes to Body
fn bytes_to_body(bytes: Bytes) -> Body {
    Body::from(Full::new(bytes))
}

use aegis_core::classifier::{
    Category, ClassificationResult, LazyNsfwClassifier, NsfwThresholdPreset, SentimentAnalyzer,
    SentimentConfig, SentimentFlag, TieredClassifier,
};
use aegis_core::content_rules::ContentRuleSet;
use aegis_core::notifications::{BlockedEvent, NotificationManager};
use aegis_core::rule_engine::{RuleAction, RuleEngine, RuleEngineResult};
use aegis_core::site_registry::SiteRegistry;
use aegis_core::time_rules::TimeRuleSet;
use aegis_storage::{Action, Database};

use crate::state_cache::StateCache;

use crate::domains::is_llm_domain;
use crate::extractor::{extract_prompt, PromptInfo};
use crate::image_extractor::{
    extract_image_from_binary, extract_images_from_json, extract_images_from_multipart,
};

/// Checks if a request is a WebSocket upgrade request.
fn is_websocket_upgrade(req: &Request<Body>) -> bool {
    req.headers()
        .get(hyper::header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false)
}

/// Block page HTML template.
const BLOCK_PAGE_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>Blocked by Aegis</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
        }
        .container {
            text-align: center;
            padding: 2rem;
            max-width: 500px;
        }
        .shield {
            font-size: 4rem;
            margin-bottom: 1rem;
        }
        h1 {
            margin: 0 0 1rem 0;
            font-size: 2rem;
        }
        p {
            margin: 0.5rem 0;
            opacity: 0.9;
        }
        .reason {
            background: rgba(255,255,255,0.2);
            padding: 1rem;
            border-radius: 8px;
            margin-top: 1rem;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="shield">🛡️</div>
        <h1>Content Blocked</h1>
        <p>This request was blocked by Aegis protection.</p>
        <div class="reason">
            <p><strong>Reason:</strong> {{REASON}}</p>
            <p><strong>Service:</strong> {{SERVICE}}</p>
        </div>
    </div>
</body>
</html>"#;

/// Callback for handling classification results.
pub type OnBlockCallback = Arc<dyn Fn(&PromptInfo, &RuleEngineResult) + Send + Sync>;
pub type OnAllowCallback = Arc<dyn Fn(&PromptInfo, &RuleEngineResult) + Send + Sync>;

/// Shared filtering state that can be controlled by ProfileProxyController.
///
/// When filtering is disabled (e.g., parent profile is active), all requests
/// pass through without classification.
///
/// Also holds the shared rule engine that can be updated when profile rules change.
#[derive(Clone)]
pub struct FilteringState {
    /// Whether filtering is enabled (fallback when no state_cache).
    enabled: Arc<AtomicBool>,
    /// Current profile name (for logging).
    profile_name: Arc<RwLock<Option<String>>>,
    /// Current profile ID (for sentiment flagging).
    profile_id: Arc<RwLock<Option<i64>>>,
    /// Shared rule engine that can be updated when rules change.
    rule_engine: Arc<RwLock<RuleEngine>>,
    /// Sentiment analyzer for emotional content flagging.
    sentiment_analyzer: Arc<RwLock<Option<SentimentAnalyzer>>>,
    /// Optional state cache for reading protection state from database (F032).
    /// When set, is_enabled() will read from the database cache instead of AtomicBool.
    state_cache: Option<Arc<StateCache>>,
    /// NSFW image threshold preset (F033).
    nsfw_threshold: Arc<RwLock<NsfwThresholdPreset>>,
    /// Whether image filtering is enabled (F033).
    image_filtering_enabled: Arc<AtomicBool>,
}

impl std::fmt::Debug for FilteringState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilteringState")
            .field("enabled", &self.enabled.load(Ordering::SeqCst))
            .field("profile_name", &*self.profile_name.read())
            .field("profile_id", &*self.profile_id.read())
            .field(
                "sentiment_enabled",
                &self.sentiment_analyzer.read().is_some(),
            )
            .field("nsfw_threshold", &*self.nsfw_threshold.read())
            .field(
                "image_filtering_enabled",
                &self.image_filtering_enabled.load(Ordering::SeqCst),
            )
            .finish()
    }
}

impl Default for FilteringState {
    fn default() -> Self {
        Self::new()
    }
}

impl FilteringState {
    /// Creates a new filtering state with filtering enabled and default rules.
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(true)),
            profile_name: Arc::new(RwLock::new(None)),
            profile_id: Arc::new(RwLock::new(None)),
            rule_engine: Arc::new(RwLock::new(RuleEngine::with_defaults())),
            sentiment_analyzer: Arc::new(RwLock::new(None)),
            state_cache: None,
            nsfw_threshold: Arc::new(RwLock::new(NsfwThresholdPreset::default())),
            image_filtering_enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Creates a new filtering state with a custom rule engine.
    pub fn with_rule_engine(rule_engine: RuleEngine) -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(true)),
            profile_name: Arc::new(RwLock::new(None)),
            profile_id: Arc::new(RwLock::new(None)),
            rule_engine: Arc::new(RwLock::new(rule_engine)),
            sentiment_analyzer: Arc::new(RwLock::new(None)),
            state_cache: None,
            nsfw_threshold: Arc::new(RwLock::new(NsfwThresholdPreset::default())),
            image_filtering_enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Creates a new filtering state with sentiment analysis enabled.
    pub fn with_sentiment_analysis(config: SentimentConfig) -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(true)),
            profile_name: Arc::new(RwLock::new(None)),
            profile_id: Arc::new(RwLock::new(None)),
            rule_engine: Arc::new(RwLock::new(RuleEngine::with_defaults())),
            sentiment_analyzer: Arc::new(RwLock::new(Some(SentimentAnalyzer::new(config)))),
            state_cache: None,
            nsfw_threshold: Arc::new(RwLock::new(NsfwThresholdPreset::default())),
            image_filtering_enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Sets the state cache for database-backed protection state (F032).
    pub fn set_state_cache(&self, _cache: Arc<StateCache>) {
        // Note: This requires interior mutability - we'll need to change the field type
        // For now, use a new constructor instead
    }

    /// Creates a filtering state with database-backed state cache (F032).
    pub fn with_state_cache(db: Arc<Database>) -> Self {
        let cache = StateCache::new(db);
        Self {
            enabled: Arc::new(AtomicBool::new(true)),
            profile_name: Arc::new(RwLock::new(None)),
            profile_id: Arc::new(RwLock::new(None)),
            rule_engine: Arc::new(RwLock::new(RuleEngine::with_defaults())),
            sentiment_analyzer: Arc::new(RwLock::new(None)),
            state_cache: Some(Arc::new(cache)),
            nsfw_threshold: Arc::new(RwLock::new(NsfwThresholdPreset::default())),
            image_filtering_enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Creates a filtering state with rule engine and state cache (F032).
    pub fn with_rule_engine_and_cache(rule_engine: RuleEngine, db: Arc<Database>) -> Self {
        let cache = StateCache::new(db);
        Self {
            enabled: Arc::new(AtomicBool::new(true)),
            profile_name: Arc::new(RwLock::new(None)),
            profile_id: Arc::new(RwLock::new(None)),
            rule_engine: Arc::new(RwLock::new(rule_engine)),
            sentiment_analyzer: Arc::new(RwLock::new(None)),
            state_cache: Some(Arc::new(cache)),
            nsfw_threshold: Arc::new(RwLock::new(NsfwThresholdPreset::default())),
            image_filtering_enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Returns whether filtering is enabled.
    /// If a state cache is configured (F032), reads from database.
    /// Otherwise, reads from local AtomicBool.
    pub fn is_enabled(&self) -> bool {
        if let Some(ref cache) = self.state_cache {
            // Poll for updates and return cached value
            cache.poll();
            cache.is_filtering_enabled()
        } else {
            self.enabled.load(Ordering::SeqCst)
        }
    }

    /// Enables filtering.
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
        tracing::info!("Filtering enabled");
    }

    /// Disables filtering.
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
        tracing::info!("Filtering disabled");
    }

    /// Sets the filtering state.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    /// Sets the current profile name.
    pub fn set_profile(&self, name: Option<String>) {
        *self.profile_name.write() = name;
    }

    /// Sets the current profile name and ID.
    pub fn set_profile_with_id(&self, name: Option<String>, id: Option<i64>) {
        *self.profile_name.write() = name;
        *self.profile_id.write() = id;
    }

    /// Returns the current profile name.
    pub fn profile_name(&self) -> Option<String> {
        self.profile_name.read().clone()
    }

    /// Returns the current profile ID.
    pub fn profile_id(&self) -> Option<i64> {
        *self.profile_id.read()
    }

    /// Returns a reference to the shared rule engine.
    pub fn rule_engine(&self) -> &Arc<RwLock<RuleEngine>> {
        &self.rule_engine
    }

    /// Returns a reference to the sentiment analyzer.
    pub fn sentiment_analyzer(&self) -> &Arc<RwLock<Option<SentimentAnalyzer>>> {
        &self.sentiment_analyzer
    }

    /// Enables sentiment analysis with the given configuration.
    pub fn enable_sentiment_analysis(&self, config: SentimentConfig) {
        *self.sentiment_analyzer.write() = Some(SentimentAnalyzer::new(config));
        tracing::info!("Sentiment analysis enabled");
    }

    /// Disables sentiment analysis.
    pub fn disable_sentiment_analysis(&self) {
        *self.sentiment_analyzer.write() = None;
        tracing::info!("Sentiment analysis disabled");
    }

    /// Returns whether sentiment analysis is enabled.
    pub fn is_sentiment_enabled(&self) -> bool {
        self.sentiment_analyzer.read().is_some()
    }

    /// Returns whether image filtering is enabled (F033).
    pub fn is_image_filtering_enabled(&self) -> bool {
        self.image_filtering_enabled.load(Ordering::SeqCst)
    }

    /// Enables image filtering (F033).
    pub fn enable_image_filtering(&self) {
        self.image_filtering_enabled.store(true, Ordering::SeqCst);
        tracing::info!("Image filtering enabled");
    }

    /// Disables image filtering (F033).
    pub fn disable_image_filtering(&self) {
        self.image_filtering_enabled.store(false, Ordering::SeqCst);
        tracing::info!("Image filtering disabled");
    }

    /// Returns the current NSFW threshold preset (F033).
    pub fn nsfw_threshold(&self) -> NsfwThresholdPreset {
        *self.nsfw_threshold.read()
    }

    /// Sets the NSFW threshold preset (F033).
    pub fn set_nsfw_threshold(&self, preset: NsfwThresholdPreset) {
        *self.nsfw_threshold.write() = preset;
        tracing::info!("NSFW threshold set to: {:?}", preset);
    }

    /// Sets the NSFW threshold from a profile age (F033).
    pub fn set_nsfw_threshold_from_age(&self, age: u8) {
        let preset = NsfwThresholdPreset::from_age(age);
        self.set_nsfw_threshold(preset);
    }

    /// Updates the rule engine with new time and content rules.
    ///
    /// Call this when a profile's rules are modified in the UI or when
    /// switching to a different profile.
    pub fn update_rules(&self, time_rules: TimeRuleSet, content_rules: ContentRuleSet) {
        let mut engine = self.rule_engine.write();
        engine.time_rules = time_rules;
        engine.content_rules = content_rules;
        tracing::info!("Rule engine updated with new rules");
    }

    /// Replaces the entire rule engine.
    pub fn set_rule_engine(&self, new_engine: RuleEngine) {
        let mut engine = self.rule_engine.write();
        *engine = new_engine;
        tracing::info!("Rule engine replaced");
    }
}

/// Handler configuration.
#[derive(Clone)]
pub struct HandlerConfig {
    /// The classifier for content analysis.
    pub classifier: Arc<RwLock<TieredClassifier>>,
    /// Optional notification manager.
    pub notifications: Option<Arc<NotificationManager>>,
    /// Callback when a request is blocked.
    pub on_block: Option<OnBlockCallback>,
    /// Callback when a request is allowed.
    pub on_allow: Option<OnAllowCallback>,
    /// Shared filtering state (controlled by ProfileProxyController).
    /// Also contains the shared rule engine.
    pub filtering_state: FilteringState,
    /// Optional database for event logging.
    pub database: Option<Arc<Database>>,
    /// Lazy-loaded NSFW image classifier (F033).
    pub nsfw_classifier: Arc<RwLock<LazyNsfwClassifier>>,
    /// Site registry for checking image gen domains (F033).
    pub site_registry: Arc<SiteRegistry>,
}

impl std::fmt::Debug for HandlerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerConfig")
            .field("classifier", &"TieredClassifier")
            .field("notifications", &self.notifications.is_some())
            .field("on_block", &self.on_block.is_some())
            .field("on_allow", &self.on_allow.is_some())
            .field("filtering_state", &self.filtering_state)
            .field("database", &self.database.is_some())
            .field("nsfw_classifier", &"LazyNsfwClassifier")
            .field("site_registry", &"SiteRegistry")
            .finish()
    }
}

/// HTTP handler for the MITM proxy.
#[derive(Clone, Debug)]
pub struct ProxyHandler {
    config: HandlerConfig,
}

impl ProxyHandler {
    /// Creates a new proxy handler with the given configuration.
    pub fn new(config: HandlerConfig) -> Self {
        Self { config }
    }

    /// Creates a handler with default classifier and rules.
    ///
    /// Uses community rules for classification by default.
    /// Filtering is enabled by default.
    pub fn with_defaults() -> Self {
        Self::new(HandlerConfig {
            // Use default classifier which includes community rules
            classifier: Arc::new(RwLock::new(TieredClassifier::with_defaults())),
            notifications: Some(Arc::new(NotificationManager::new())),
            on_block: None,
            on_allow: None,
            filtering_state: FilteringState::new(),
            database: None,
            nsfw_classifier: Arc::new(RwLock::new(LazyNsfwClassifier::with_defaults())),
            site_registry: Arc::new(SiteRegistry::with_defaults()),
        })
    }

    /// Creates a handler with the given filtering state.
    ///
    /// This allows external control of filtering (e.g., by ProfileProxyController).
    /// The rule engine is obtained from the filtering state.
    pub fn with_filtering_state(filtering_state: FilteringState) -> Self {
        Self::new(HandlerConfig {
            classifier: Arc::new(RwLock::new(TieredClassifier::with_defaults())),
            notifications: Some(Arc::new(NotificationManager::new())),
            on_block: None,
            on_allow: None,
            filtering_state,
            database: None,
            nsfw_classifier: Arc::new(RwLock::new(LazyNsfwClassifier::with_defaults())),
            site_registry: Arc::new(SiteRegistry::with_defaults()),
        })
    }

    /// Returns the filtering state.
    pub fn filtering_state(&self) -> &FilteringState {
        &self.config.filtering_state
    }

    /// Returns whether filtering is currently enabled.
    pub fn is_filtering_enabled(&self) -> bool {
        self.config.filtering_state.is_enabled()
    }

    /// Sets the callback for blocked requests.
    pub fn on_block<F>(mut self, callback: F) -> Self
    where
        F: Fn(&PromptInfo, &RuleEngineResult) + Send + Sync + 'static,
    {
        self.config.on_block = Some(Arc::new(callback));
        self
    }

    /// Sets the callback for allowed requests.
    pub fn on_allow<F>(mut self, callback: F) -> Self
    where
        F: Fn(&PromptInfo, &RuleEngineResult) + Send + Sync + 'static,
    {
        self.config.on_allow = Some(Arc::new(callback));
        self
    }

    /// Processes a request body and returns classification result.
    fn classify_prompt(&self, prompt: &str) -> ClassificationResult {
        self.config.classifier.write().classify(prompt)
    }

    /// Evaluates rules against classification.
    fn evaluate_rules(&self, classification: &ClassificationResult) -> RuleEngineResult {
        self.config
            .filtering_state
            .rule_engine
            .read()
            .evaluate_now(classification)
    }

    /// Records an event to the database if configured.
    fn record_event(
        &self,
        prompt: &PromptInfo,
        classification: &ClassificationResult,
        action: Action,
    ) {
        if let Some(ref db) = self.config.database {
            // Get the primary category from classification
            let category = classification.matches.first().map(|m| m.category);
            let confidence = classification.matches.first().map(|m| m.confidence);

            // Log event (this also updates daily stats)
            if let Err(e) = db.log_event(
                &prompt.text,
                category,
                confidence,
                action,
                Some(prompt.service.clone()),
            ) {
                tracing::warn!("Failed to record event: {}", e);
            }
        }
    }

    /// Analyzes sentiment and records flagged events.
    ///
    /// This runs after classification to detect emotional content that may
    /// warrant parental review. Flagged events don't affect blocking decisions.
    fn analyze_and_flag_sentiment(&self, prompt: &PromptInfo) {
        // Check if we have a profile ID (required for flagging)
        let profile_id = match self.config.filtering_state.profile_id() {
            Some(id) => id,
            None => {
                tracing::debug!("No profile ID set, skipping sentiment analysis");
                return;
            }
        };

        // Get the sentiment analyzer
        let mut analyzer_guard = self.config.filtering_state.sentiment_analyzer.write();
        let analyzer = match analyzer_guard.as_mut() {
            Some(a) => a,
            None => {
                tracing::debug!("Sentiment analysis not enabled");
                return;
            }
        };

        // Analyze the prompt
        let result = analyzer.analyze(&prompt.text);

        // Record any flags to the database
        if result.has_flags() {
            if let Some(ref db) = self.config.database {
                for flag in &result.flags {
                    let flag_type = match flag.flag {
                        SentimentFlag::Distress => "distress",
                        SentimentFlag::CrisisIndicator => "crisis_indicator",
                        SentimentFlag::Bullying => "bullying",
                        SentimentFlag::NegativeSentiment => "negative_sentiment",
                    };

                    if let Err(e) = db.log_flagged_event(
                        profile_id,
                        flag_type,
                        flag.confidence,
                        &prompt.text,
                        Some(prompt.service.clone()),
                        flag.matched_phrases.clone(),
                    ) {
                        tracing::warn!("Failed to record flagged event: {}", e);
                    } else {
                        tracing::info!(
                            "Flagged {} content from {} (confidence: {:.2})",
                            flag.flag.name(),
                            prompt.service,
                            flag.confidence
                        );
                    }
                }
            }
        }
    }

    /// Checks if a host is an image generation domain (F033).
    fn is_image_gen_domain(&self, host: &str) -> bool {
        self.config.site_registry.is_image_gen_domain(host)
    }

    /// Creates a block response for NSFW image content (F033).
    fn create_image_block_response(&self, service: &str) -> Response<Body> {
        self.create_block_response("NSFW/explicit image content detected", service)
    }

    /// Creates a block response.
    fn create_block_response(&self, reason: &str, service: &str) -> Response<Body> {
        let html = BLOCK_PAGE_HTML
            .replace("{{REASON}}", reason)
            .replace("{{SERVICE}}", service);

        Response::builder()
            .status(403)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("X-Aegis-Blocked", "true")
            .body(bytes_to_body(Bytes::from(html)))
            .unwrap()
    }

    /// Handles a request to an LLM domain.
    async fn handle_llm_request(&self, host: &str, req: Request<Body>) -> RequestOrResponse {
        let uri = req.uri().clone();
        let path = uri.path();
        let method = req.method().clone();

        // Only process POST requests (API calls)
        if method != hyper::Method::POST {
            return RequestOrResponse::Request(req);
        }

        // Check if filtering is enabled (profile-aware)
        if !self.config.filtering_state.is_enabled() {
            let profile = self.config.filtering_state.profile_name();
            tracing::debug!(
                "Filtering disabled (profile: {:?}), passing through request to {}",
                profile,
                host
            );
            return RequestOrResponse::Request(req);
        }

        tracing::info!("HTTP POST to LLM domain: {}{}", host, path);

        // Read the body
        let (parts, body) = req.into_parts();
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                tracing::warn!("Failed to read request body: {}", e);
                return RequestOrResponse::Request(Request::from_parts(parts, Body::empty()));
            }
        };

        // Check for multipart image uploads to image gen domains (F033)
        if self.is_image_gen_domain(host)
            && self.config.filtering_state.is_image_filtering_enabled()
        {
            let content_type = parts
                .headers
                .get(hyper::header::CONTENT_TYPE)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");

            // Handle multipart/form-data uploads
            if content_type.starts_with("multipart/form-data") {
                if let Some(boundary) = extract_multipart_boundary(content_type) {
                    let images = extract_images_from_multipart(&body_bytes, &boundary);

                    if !images.is_empty() {
                        let threshold = self.config.filtering_state.nsfw_threshold().threshold();
                        tracing::info!(
                            "Checking {} uploaded image(s) to {} for NSFW content",
                            images.len(),
                            host
                        );

                        for (field_name, img) in images {
                            let mut classifier = self.config.nsfw_classifier.write();
                            if let Some(Ok(result)) = classifier.classify_bytes(&img.data) {
                                tracing::debug!(
                                    "Upload image '{}' NSFW score: {:.3} (threshold: {:.3})",
                                    field_name,
                                    result.nsfw_probability,
                                    threshold
                                );

                                if result.is_nsfw(threshold) {
                                    let service_name = self.config.site_registry.service_name(host);

                                    tracing::warn!(
                                        "Blocked NSFW image upload '{}' to {}: score {:.3} exceeds threshold {:.3}",
                                        field_name,
                                        service_name,
                                        result.nsfw_probability,
                                        threshold
                                    );

                                    // Log the blocked event
                                    if let Some(ref db) = self.config.database {
                                        let _ = db.log_event(
                                            &format!("[NSFW image upload blocked: {}]", field_name),
                                            Some(Category::Adult),
                                            Some(result.nsfw_probability),
                                            Action::Blocked,
                                            Some(service_name.to_string()),
                                        );
                                    }

                                    // Send notification
                                    if let Some(ref notifications) = self.config.notifications {
                                        let event = BlockedEvent::new(
                                            Some(service_name.to_string()),
                                            Some(Category::Adult),
                                            Some("NSFW image upload blocked".to_string()),
                                            false,
                                        );
                                        let _ = notifications.notify_block(&event);
                                    }

                                    return RequestOrResponse::Response(
                                        self.create_block_response(
                                            "NSFW/explicit image upload detected",
                                            service_name,
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // Extract prompt
        let prompt_info = match extract_prompt(host, path, &body_bytes) {
            Some(info) => info,
            None => {
                // No prompt extracted, forward request
                tracing::info!(
                    "No prompt extracted from {}{} ({} bytes)",
                    host,
                    path,
                    body_bytes.len()
                );
                return RequestOrResponse::Request(Request::from_parts(
                    parts,
                    bytes_to_body(body_bytes),
                ));
            }
        };

        tracing::info!(
            "Extracted prompt from {}: {} chars",
            prompt_info.service,
            prompt_info.text.len()
        );

        // Classify the prompt
        let classification = self.classify_prompt(&prompt_info.text);

        // Analyze sentiment for parental review flagging (runs regardless of blocking)
        self.analyze_and_flag_sentiment(&prompt_info);

        // Evaluate rules
        let result = self.evaluate_rules(&classification);

        match result.action {
            RuleAction::Block => {
                let reason = result
                    .source
                    .rule_name()
                    .unwrap_or("Policy violation")
                    .to_string();

                tracing::info!(
                    "Blocked request to {} - reason: {}",
                    prompt_info.service,
                    reason
                );

                // Record event to database
                self.record_event(&prompt_info, &classification, Action::Blocked);

                // Send notification if enabled
                if let Some(notifications) = &self.config.notifications {
                    let event = BlockedEvent::from_rule_source(
                        &result.source,
                        Some(prompt_info.service.clone()),
                    );
                    let _ = notifications.notify_block(&event);
                }

                // Call on_block callback
                if let Some(callback) = &self.config.on_block {
                    callback(&prompt_info, &result);
                }

                // Return block page
                RequestOrResponse::Response(
                    self.create_block_response(&reason, &prompt_info.service),
                )
            }
            RuleAction::Warn => {
                tracing::info!(
                    "Warned request to {} - reason: {:?}",
                    prompt_info.service,
                    result.source.rule_name()
                );

                // Record event to database
                self.record_event(&prompt_info, &classification, Action::Flagged);

                // Call on_allow callback (warn still allows)
                if let Some(callback) = &self.config.on_allow {
                    callback(&prompt_info, &result);
                }

                // Forward the request with warning header
                let mut req = Request::from_parts(parts, bytes_to_body(body_bytes));
                req.headers_mut()
                    .insert("X-Aegis-Warning", "true".parse().unwrap());
                RequestOrResponse::Request(req)
            }
            RuleAction::Allow => {
                tracing::debug!("Allowed request to {}", prompt_info.service);

                // Record event to database
                self.record_event(&prompt_info, &classification, Action::Allowed);

                // Call on_allow callback
                if let Some(callback) = &self.config.on_allow {
                    callback(&prompt_info, &result);
                }

                // Forward the request
                RequestOrResponse::Request(Request::from_parts(parts, bytes_to_body(body_bytes)))
            }
        }
    }

    /// Extracts host from request URI or Host header.
    fn extract_host(req: &Request<Body>) -> Option<String> {
        // Try to get from URI first
        if let Some(host) = req.uri().host() {
            return Some(host.to_string());
        }

        // Fall back to Host header
        req.headers()
            .get(hyper::header::HOST)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.split(':').next().unwrap_or(s).to_string())
    }
}

impl HttpHandler for ProxyHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        mut req: Request<Body>,
    ) -> RequestOrResponse {
        let host = match Self::extract_host(&req) {
            Some(h) => h,
            None => return RequestOrResponse::Request(req),
        };

        // For all WebSocket upgrades, strip compression extension
        // This prevents protocol errors when proxying compressed WebSocket messages
        // We do this for all domains because the proxy doesn't handle permessage-deflate
        if is_websocket_upgrade(&req) {
            tracing::info!("WebSocket upgrade request to {}", host);
            req.headers_mut().remove("sec-websocket-extensions");
        }

        // Only intercept LLM domains
        if !is_llm_domain(&host) {
            return RequestOrResponse::Request(req);
        }

        tracing::info!(
            "Intercepting {} request to LLM domain: {}",
            req.method(),
            host
        );

        self.handle_llm_request(&host, req).await
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        // Check if image filtering is enabled
        if !self.config.filtering_state.is_image_filtering_enabled() {
            return res;
        }

        // Check if filtering is enabled at all
        if !self.config.filtering_state.is_enabled() {
            return res;
        }

        // Get content type before moving res
        let content_type = res
            .headers()
            .get(hyper::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_string();

        // Only process JSON or image responses that might contain generated images
        // Note: We can't easily get the host in handle_response, so we filter based on content type
        if !content_type.starts_with("application/json") && !content_type.starts_with("image/") {
            return res;
        }

        // Read response body
        let (parts, body) = res.into_parts();
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                tracing::warn!("Failed to read response body: {}", e);
                return Response::from_parts(parts, Body::empty());
            }
        };

        // Get the NSFW threshold
        let threshold = self.config.filtering_state.nsfw_threshold().threshold();

        // Extract and classify images based on content type
        let mut nsfw_detected = false;
        let mut nsfw_score: f32 = 0.0;

        if content_type.starts_with("application/json") {
            // JSON response - extract base64 images
            let images = extract_images_from_json(&body_bytes);
            if !images.is_empty() {
                tracing::info!(
                    "Extracted {} image(s) from JSON response for NSFW check",
                    images.len()
                );

                for img in images {
                    let mut classifier = self.config.nsfw_classifier.write();
                    match classifier.classify_bytes(&img.data) {
                        Some(Ok(result)) => {
                            tracing::info!(
                                "Image {} NSFW score: {:.3} (threshold: {:.3})",
                                img.source_path,
                                result.nsfw_probability,
                                threshold
                            );

                            if result.is_nsfw(threshold) {
                                nsfw_detected = true;
                                nsfw_score = nsfw_score.max(result.nsfw_probability);
                                tracing::warn!(
                                    "NSFW image detected: score {:.3} exceeds threshold {:.3}",
                                    result.nsfw_probability,
                                    threshold
                                );
                                break; // One NSFW image is enough to block
                            }
                        }
                        Some(Err(e)) => {
                            tracing::warn!(
                                "NSFW classification error for {}: {}",
                                img.source_path,
                                e
                            );
                        }
                        None => {
                            tracing::debug!(
                                "NSFW classifier not available - skipping image check for {}",
                                img.source_path
                            );
                        }
                    }
                }
            }
        } else if content_type.starts_with("image/") {
            // Binary image response
            if let Some(img) = extract_image_from_binary(&body_bytes, Some(&content_type)) {
                tracing::info!(
                    "Extracted binary image ({} bytes) for NSFW check",
                    img.data.len()
                );

                let mut classifier = self.config.nsfw_classifier.write();
                match classifier.classify_bytes(&img.data) {
                    Some(Ok(result)) => {
                        tracing::info!(
                            "Binary image NSFW score: {:.3} (threshold: {:.3})",
                            result.nsfw_probability,
                            threshold
                        );

                        if result.is_nsfw(threshold) {
                            nsfw_detected = true;
                            nsfw_score = result.nsfw_probability;
                            tracing::warn!(
                                "NSFW image detected: score {:.3} exceeds threshold {:.3}",
                                result.nsfw_probability,
                                threshold
                            );
                        }
                    }
                    Some(Err(e)) => {
                        tracing::warn!("NSFW classification error: {}", e);
                    }
                    None => {
                        tracing::debug!(
                            "NSFW classifier not available - skipping binary image check"
                        );
                    }
                }
            }
        }

        // Block if NSFW content detected
        if nsfw_detected {
            let service_name = "Image Service";

            // Log the blocked event
            if let Some(ref db) = self.config.database {
                if let Err(e) = db.log_event(
                    "[NSFW Image blocked]",
                    Some(Category::Adult),
                    Some(nsfw_score),
                    Action::Blocked,
                    Some(service_name.to_string()),
                ) {
                    tracing::warn!("Failed to log NSFW image block event: {}", e);
                }
            }

            // Send notification
            if let Some(ref notifications) = self.config.notifications {
                let event = BlockedEvent::new(
                    Some(service_name.to_string()),
                    Some(Category::Adult),
                    Some("NSFW image content detected".to_string()),
                    false,
                );
                let _ = notifications.notify_block(&event);
            }

            tracing::info!("Blocked NSFW image response (score: {:.3})", nsfw_score);

            return self.create_image_block_response(service_name);
        }

        // Pass through unchanged
        Response::from_parts(parts, bytes_to_body(body_bytes))
    }
}

impl WebSocketHandler for ProxyHandler {
    fn handle_message(
        &mut self,
        ctx: &WebSocketContext,
        message: Message,
    ) -> impl std::future::Future<Output = Option<Message>> + Send {
        let classifier = self.config.classifier.clone();
        let filtering_state = self.config.filtering_state.clone();
        let notifications = self.config.notifications.clone();

        // Only inspect client-to-server messages (outgoing prompts)
        // Server-to-client messages (responses) pass through unchanged
        let host = match ctx {
            WebSocketContext::ClientToServer { dst, .. } => {
                Some(dst.host().unwrap_or("unknown").to_string())
            }
            WebSocketContext::ServerToClient { .. } => None,
        };

        async move {
            // Skip server-to-client (responses) - pass through unchanged
            let host = match host {
                Some(h) => h,
                None => return Some(message),
            };

            // Only inspect LLM domain WebSocket traffic
            if !is_llm_domain(&host) {
                return Some(message);
            }

            // Check if filtering is enabled (profile-aware)
            if !filtering_state.is_enabled() {
                tracing::debug!(
                    "Filtering disabled, passing through WebSocket message to {}",
                    host
                );
                return Some(message);
            }

            // Only inspect text messages (JSON payloads)
            let text = match &message {
                Message::Text(t) => {
                    let s = t.to_string();
                    tracing::info!("WebSocket text message from {} ({} bytes)", host, s.len());
                    s
                }
                Message::Binary(b) => {
                    tracing::debug!("WebSocket binary message from {} ({} bytes)", host, b.len());
                    return Some(message);
                }
                _ => return Some(message),
            };

            // Try to extract prompt from WebSocket message
            // ChatGPT WebSocket messages contain JSON with message content
            if let Some(prompt) = extract_websocket_prompt(&text) {
                tracing::info!(
                    "WebSocket prompt extracted from {}: {} chars",
                    host,
                    prompt.len()
                );

                // Classify the prompt
                let classification = classifier.write().classify(&prompt);

                // Evaluate rules using the shared rule engine
                let result = filtering_state
                    .rule_engine
                    .read()
                    .evaluate_now(&classification);

                match result.action {
                    RuleAction::Block => {
                        let reason = result.source.rule_name().unwrap_or("Policy violation");

                        tracing::info!(
                            "Blocked WebSocket message to {} - reason: {}",
                            host,
                            reason
                        );

                        // Send notification
                        if let Some(ref notif) = notifications {
                            let event = BlockedEvent::from_rule_source(
                                &result.source,
                                Some(crate::domains::service_name(&host).to_string()),
                            );
                            let _ = notif.notify_block(&event);
                        }

                        // Block by returning None
                        return None;
                    }
                    RuleAction::Warn => {
                        tracing::info!("Warned WebSocket message to {}", host);
                    }
                    RuleAction::Allow => {
                        tracing::debug!("Allowed WebSocket message to {}", host);
                    }
                }
            }

            Some(message)
        }
    }
}

/// Extracts the boundary from a multipart/form-data content-type header.
fn extract_multipart_boundary(content_type: &str) -> Option<String> {
    // Content-Type: multipart/form-data; boundary=----WebKitFormBoundary...
    content_type.split(';').find_map(|part| {
        part.trim()
            .strip_prefix("boundary=")
            .map(|b| b.trim_matches('"').to_string())
    })
}

/// Extracts prompt text from a WebSocket message (usually JSON).
fn extract_websocket_prompt(text: &str) -> Option<String> {
    // Try to parse as JSON and extract common prompt fields
    let json: serde_json::Value = serde_json::from_str(text).ok()?;

    // ChatGPT WebSocket format: look for message content
    // Try various known paths

    // Path: .messages[].content.parts[]
    if let Some(messages) = json.get("messages").and_then(|m| m.as_array()) {
        let mut prompts = Vec::new();
        for msg in messages {
            if let Some(content) = msg.get("content") {
                if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                    for part in parts {
                        if let Some(text) = part.as_str() {
                            prompts.push(text.to_string());
                        }
                    }
                } else if let Some(text) = content.as_str() {
                    prompts.push(text.to_string());
                }
            }
        }
        if !prompts.is_empty() {
            return Some(prompts.join("\n"));
        }
    }

    // Path: .message.content.parts[]
    if let Some(message) = json.get("message") {
        if let Some(content) = message.get("content") {
            if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                let mut prompts = Vec::new();
                for part in parts {
                    if let Some(text) = part.as_str() {
                        prompts.push(text.to_string());
                    }
                }
                if !prompts.is_empty() {
                    return Some(prompts.join("\n"));
                }
            }
        }
    }

    // Path: .prompt or .text or .content (simple cases)
    if let Some(prompt) = json.get("prompt").and_then(|p| p.as_str()) {
        return Some(prompt.to_string());
    }
    if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
        return Some(text.to_string());
    }
    if let Some(content) = json.get("content").and_then(|c| c.as_str()) {
        return Some(content.to_string());
    }

    // Path: .action (ChatGPT specific)
    if let Some(action) = json.get("action").and_then(|a| a.as_str()) {
        if action == "next" || action == "continue" {
            // This is a prompt submission
            if let Some(messages) = json.get("messages").and_then(|m| m.as_array()) {
                let mut prompts = Vec::new();
                for msg in messages {
                    if let Some(content) = msg
                        .get("content")
                        .and_then(|c| c.get("parts"))
                        .and_then(|p| p.as_array())
                    {
                        for part in content {
                            if let Some(text) = part.as_str() {
                                prompts.push(text.to_string());
                            }
                        }
                    }
                }
                if !prompts.is_empty() {
                    return Some(prompts.join("\n"));
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_config_debug() {
        let config = HandlerConfig {
            classifier: Arc::new(RwLock::new(TieredClassifier::keyword_only())),
            notifications: None,
            on_block: None,
            on_allow: None,
            filtering_state: FilteringState::new(),
            database: None,
            nsfw_classifier: Arc::new(RwLock::new(LazyNsfwClassifier::with_defaults())),
            site_registry: Arc::new(SiteRegistry::with_defaults()),
        };
        let debug = format!("{:?}", config);
        assert!(debug.contains("HandlerConfig"));
    }

    #[test]
    fn filtering_state_default_enabled() {
        let state = FilteringState::new();
        assert!(state.is_enabled());
        assert!(state.profile_name().is_none());
    }

    // ==================== Image Filtering Tests (F033) ====================

    #[test]
    fn filtering_state_image_filtering_default_enabled() {
        let state = FilteringState::new();
        assert!(state.is_image_filtering_enabled());
        assert_eq!(state.nsfw_threshold(), NsfwThresholdPreset::Teen);
    }

    #[test]
    fn filtering_state_set_nsfw_threshold() {
        let state = FilteringState::new();

        state.set_nsfw_threshold(NsfwThresholdPreset::Child);
        assert_eq!(state.nsfw_threshold(), NsfwThresholdPreset::Child);
        assert_eq!(state.nsfw_threshold().threshold(), 0.5);

        state.set_nsfw_threshold(NsfwThresholdPreset::Adult);
        assert_eq!(state.nsfw_threshold(), NsfwThresholdPreset::Adult);
        assert_eq!(state.nsfw_threshold().threshold(), 0.85);

        state.set_nsfw_threshold(NsfwThresholdPreset::Custom(0.6));
        assert_eq!(state.nsfw_threshold().threshold(), 0.6);
    }

    #[test]
    fn filtering_state_set_nsfw_threshold_from_age() {
        let state = FilteringState::new();

        state.set_nsfw_threshold_from_age(8);
        assert_eq!(state.nsfw_threshold(), NsfwThresholdPreset::Child);

        state.set_nsfw_threshold_from_age(15);
        assert_eq!(state.nsfw_threshold(), NsfwThresholdPreset::Teen);

        state.set_nsfw_threshold_from_age(25);
        assert_eq!(state.nsfw_threshold(), NsfwThresholdPreset::Adult);
    }

    #[test]
    fn filtering_state_disable_enable_image_filtering() {
        let state = FilteringState::new();
        assert!(state.is_image_filtering_enabled());

        state.disable_image_filtering();
        assert!(!state.is_image_filtering_enabled());

        state.enable_image_filtering();
        assert!(state.is_image_filtering_enabled());
    }

    #[test]
    fn extract_multipart_boundary_basic() {
        let content_type = "multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW";
        let boundary = extract_multipart_boundary(content_type);
        assert_eq!(
            boundary,
            Some("----WebKitFormBoundary7MA4YWxkTrZu0gW".to_string())
        );
    }

    #[test]
    fn extract_multipart_boundary_quoted() {
        let content_type = r#"multipart/form-data; boundary="----WebKitFormBoundary""#;
        let boundary = extract_multipart_boundary(content_type);
        assert_eq!(boundary, Some("----WebKitFormBoundary".to_string()));
    }

    #[test]
    fn extract_multipart_boundary_none() {
        let content_type = "application/json";
        let boundary = extract_multipart_boundary(content_type);
        assert!(boundary.is_none());
    }

    #[test]
    fn handler_is_image_gen_domain() {
        let handler = ProxyHandler::with_defaults();

        // Image gen domains
        assert!(handler.is_image_gen_domain("api.stability.ai"));
        assert!(handler.is_image_gen_domain("cloud.leonardo.ai"));
        assert!(handler.is_image_gen_domain("api.ideogram.ai"));

        // Non-image gen domains
        assert!(!handler.is_image_gen_domain("api.openai.com")); // Text LLM
        assert!(!handler.is_image_gen_domain("chatgpt.com")); // Consumer LLM
        assert!(!handler.is_image_gen_domain("example.com")); // Unknown
    }

    #[test]
    fn filtering_state_enable_disable() {
        let state = FilteringState::new();
        assert!(state.is_enabled());

        state.disable();
        assert!(!state.is_enabled());

        state.enable();
        assert!(state.is_enabled());
    }

    #[test]
    fn filtering_state_profile_name() {
        let state = FilteringState::new();
        state.set_profile(Some("Alice".to_string()));
        assert_eq!(state.profile_name(), Some("Alice".to_string()));

        state.set_profile(None);
        assert!(state.profile_name().is_none());
    }

    #[test]
    fn handler_with_filtering_state() {
        let filtering_state = FilteringState::new();
        filtering_state.disable();

        let handler = ProxyHandler::with_filtering_state(filtering_state);
        assert!(!handler.is_filtering_enabled());
    }

    #[test]
    fn proxy_handler_new() {
        let handler = ProxyHandler::with_defaults();
        assert!(handler.config.notifications.is_some());
    }

    #[test]
    fn block_page_contains_placeholders() {
        assert!(BLOCK_PAGE_HTML.contains("{{REASON}}"));
        assert!(BLOCK_PAGE_HTML.contains("{{SERVICE}}"));
    }

    #[test]
    fn create_block_response_status_and_headers() {
        let handler = ProxyHandler::with_defaults();
        let response = handler.create_block_response("Violence detected", "ChatGPT");

        assert_eq!(response.status(), 403);
        assert_eq!(
            response.headers().get("Content-Type").unwrap(),
            "text/html; charset=utf-8"
        );
        assert_eq!(response.headers().get("X-Aegis-Blocked").unwrap(), "true");
    }

    #[test]
    fn classify_prompt_works() {
        let handler = ProxyHandler::with_defaults();
        let result = handler.classify_prompt("Hello, how are you?");
        assert!(result.matches.is_empty()); // Safe content
    }

    #[test]
    fn evaluate_rules_allows_safe() {
        use aegis_core::rule_engine::RuleEngine;

        // Create handler without time rules to avoid bedtime blocking during tests
        let filtering_state = FilteringState::with_rule_engine(RuleEngine::content_only());
        let handler = ProxyHandler::with_filtering_state(filtering_state);

        let classification = handler.classify_prompt("What is the weather today?");
        assert!(
            classification.matches.is_empty(),
            "Safe prompt should not match any rules, but matched: {:?}",
            classification.matches
        );
        let result = handler.evaluate_rules(&classification);
        assert!(
            result.should_allow(),
            "Safe prompt should be allowed, but got action: {:?}",
            result.action
        );
    }

    #[test]
    fn handler_with_callbacks() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let block_called = Arc::new(AtomicBool::new(false));
        let block_called_clone = block_called.clone();

        let _handler = ProxyHandler::with_defaults().on_block(move |_, _| {
            block_called_clone.store(true, Ordering::SeqCst);
        });

        // Note: We can't easily test the callback being called without
        // setting up a full async request context
    }

    #[test]
    fn filtering_state_sentiment_analysis() {
        let state = FilteringState::new();

        // Initially, sentiment analysis is not enabled
        assert!(!state.is_sentiment_enabled());
        assert!(state.profile_id().is_none());

        // Set profile with ID
        state.set_profile_with_id(Some("TestChild".to_string()), Some(1));
        assert_eq!(state.profile_name(), Some("TestChild".to_string()));
        assert_eq!(state.profile_id(), Some(1));

        // Enable sentiment analysis with default config
        let config = SentimentConfig::default();
        state.enable_sentiment_analysis(config);
        assert!(state.is_sentiment_enabled());

        // Verify analyzer is present and can analyze
        {
            let mut analyzer_guard = state.sentiment_analyzer.write();
            let analyzer = analyzer_guard.as_mut().expect("analyzer should be present");

            // Test with distressing content
            let result = analyzer.analyze("I feel so alone and nobody cares about me");
            assert!(result.has_flags(), "Should flag distressing content");
            assert!(
                result
                    .flags
                    .iter()
                    .any(|f| f.flag == SentimentFlag::Distress),
                "Should detect distress"
            );
        }

        // Disable sentiment analysis
        state.disable_sentiment_analysis();
        assert!(!state.is_sentiment_enabled());
    }

    #[test]
    fn filtering_state_with_sentiment_analysis() {
        // Create filtering state with sentiment analysis from the start
        let state = FilteringState::with_sentiment_analysis(SentimentConfig::default());

        assert!(state.is_enabled());
        assert!(state.is_sentiment_enabled());

        // Verify it can analyze emotional content
        {
            let mut analyzer_guard = state.sentiment_analyzer.write();
            let analyzer = analyzer_guard.as_mut().expect("analyzer should be present");

            // Test with crisis indicator content
            let result = analyzer.analyze("I don't want to be here anymore, I want to disappear");
            assert!(result.has_flags(), "Should flag crisis indicator content");
        }
    }

    #[test]
    fn sentiment_analysis_full_integration() {
        use aegis_storage::{
            models::ProfileSentimentConfig, NewProfile, ProfileImageFilteringConfig,
        };
        use std::collections::HashSet;

        // Create an in-memory database
        let db = Database::in_memory().expect("Failed to create in-memory database");

        // Create a profile with sentiment analysis enabled
        let profile = NewProfile {
            name: "TestChild".to_string(),
            os_username: None,
            time_rules: serde_json::json!({"rules": []}),
            content_rules: serde_json::json!({"rules": []}),
            enabled: true,
            sentiment_config: ProfileSentimentConfig {
                enabled: true,
                sensitivity: 0.5,
                detect_distress: true,
                detect_crisis: true,
                detect_bullying: true,
                detect_negative: true,
            },
            image_filtering_config: ProfileImageFilteringConfig::default(),
        };
        let profile_id = db
            .create_profile(profile)
            .expect("Failed to create profile");

        // Create filtering state with the profile
        let filtering_state = FilteringState::new();
        filtering_state.set_profile_with_id(Some("TestChild".to_string()), Some(profile_id));

        // Build enabled flags from config
        let mut enabled_flags = HashSet::new();
        enabled_flags.insert(SentimentFlag::Distress);
        enabled_flags.insert(SentimentFlag::CrisisIndicator);
        enabled_flags.insert(SentimentFlag::Bullying);
        enabled_flags.insert(SentimentFlag::NegativeSentiment);

        let sentiment_config = SentimentConfig {
            enabled: true,
            threshold: 0.5,
            enabled_flags,
            notify_on_flag: true,
        };
        filtering_state.enable_sentiment_analysis(sentiment_config);

        // Create handler with database via HandlerConfig
        let db_arc = Arc::new(db.clone());
        let handler = ProxyHandler::new(HandlerConfig {
            classifier: Arc::new(RwLock::new(TieredClassifier::with_defaults())),
            notifications: None,
            on_block: None,
            on_allow: None,
            filtering_state,
            database: Some(db_arc),
            nsfw_classifier: Arc::new(RwLock::new(LazyNsfwClassifier::with_defaults())),
            site_registry: Arc::new(SiteRegistry::with_defaults()),
        });

        // Simulate analyzing emotional content
        let prompt = PromptInfo::new(
            "I feel so alone and nobody cares about me. I'm so sad and hopeless.",
            "TestService",
            "/v1/chat/completions",
        );
        handler.analyze_and_flag_sentiment(&prompt);

        // Check that the flagged event was stored in the database
        let filter = aegis_storage::FlaggedEventFilter::default();
        let events = db
            .get_flagged_events(filter)
            .expect("Failed to get flagged events");

        assert!(!events.is_empty(), "Should have flagged events");
        assert_eq!(events[0].profile_id, profile_id);
        // The analyzer may flag as distress, negative_sentiment, or both
        // depending on which patterns match - just verify it's a valid flag type
        assert!(
            [
                "distress",
                "negative_sentiment",
                "crisis_indicator",
                "bullying"
            ]
            .contains(&events[0].flag_type.as_str()),
            "Should be a valid flag type, got: {}",
            events[0].flag_type
        );
        println!("Successfully flagged {} event(s):", events.len());
        for event in &events {
            println!(
                "  - {} (confidence: {:.2})",
                event.flag_type, event.confidence
            );
        }
    }
}

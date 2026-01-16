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

use aegis_core::classifier::{ClassificationResult, TieredClassifier};
use aegis_core::notifications::{BlockedEvent, NotificationManager};
use aegis_core::rule_engine::{RuleAction, RuleEngine, RuleEngineResult};

use crate::domains::is_llm_domain;
use crate::extractor::{extract_prompt, PromptInfo};

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
#[derive(Clone, Debug)]
pub struct FilteringState {
    /// Whether filtering is enabled.
    enabled: Arc<AtomicBool>,
    /// Current profile name (for logging).
    profile_name: Arc<RwLock<Option<String>>>,
}

impl Default for FilteringState {
    fn default() -> Self {
        Self::new()
    }
}

impl FilteringState {
    /// Creates a new filtering state with filtering enabled.
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(true)),
            profile_name: Arc::new(RwLock::new(None)),
        }
    }

    /// Returns whether filtering is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
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

    /// Returns the current profile name.
    pub fn profile_name(&self) -> Option<String> {
        self.profile_name.read().clone()
    }
}

/// Handler configuration.
#[derive(Clone)]
pub struct HandlerConfig {
    /// The classifier for content analysis.
    pub classifier: Arc<RwLock<TieredClassifier>>,
    /// The rule engine for policy evaluation.
    pub rule_engine: Arc<RuleEngine>,
    /// Optional notification manager.
    pub notifications: Option<Arc<NotificationManager>>,
    /// Callback when a request is blocked.
    pub on_block: Option<OnBlockCallback>,
    /// Callback when a request is allowed.
    pub on_allow: Option<OnAllowCallback>,
    /// Shared filtering state (controlled by ProfileProxyController).
    pub filtering_state: FilteringState,
}

impl std::fmt::Debug for HandlerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerConfig")
            .field("classifier", &"TieredClassifier")
            .field("rule_engine", &"RuleEngine")
            .field("notifications", &self.notifications.is_some())
            .field("on_block", &self.on_block.is_some())
            .field("on_allow", &self.on_allow.is_some())
            .field("filtering_state", &self.filtering_state)
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
            rule_engine: Arc::new(RuleEngine::with_defaults()),
            notifications: Some(Arc::new(NotificationManager::new())),
            on_block: None,
            on_allow: None,
            filtering_state: FilteringState::new(),
        })
    }

    /// Creates a handler with the given filtering state.
    ///
    /// This allows external control of filtering (e.g., by ProfileProxyController).
    pub fn with_filtering_state(filtering_state: FilteringState) -> Self {
        Self::new(HandlerConfig {
            classifier: Arc::new(RwLock::new(TieredClassifier::with_defaults())),
            rule_engine: Arc::new(RuleEngine::with_defaults()),
            notifications: Some(Arc::new(NotificationManager::new())),
            on_block: None,
            on_allow: None,
            filtering_state,
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
        self.config.rule_engine.evaluate_now(classification)
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
        // Pass through responses unchanged
        res
    }
}

impl WebSocketHandler for ProxyHandler {
    fn handle_message(
        &mut self,
        ctx: &WebSocketContext,
        message: Message,
    ) -> impl std::future::Future<Output = Option<Message>> + Send {
        let classifier = self.config.classifier.clone();
        let rule_engine = self.config.rule_engine.clone();
        let notifications = self.config.notifications.clone();
        let filtering_state = self.config.filtering_state.clone();

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

                // Evaluate rules
                let result = rule_engine.evaluate_now(&classification);

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
            rule_engine: Arc::new(RuleEngine::new()),
            notifications: None,
            on_block: None,
            on_allow: None,
            filtering_state: FilteringState::new(),
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
        let handler = ProxyHandler::with_defaults();
        let classification = handler.classify_prompt("What is the weather today?");
        let result = handler.evaluate_rules(&classification);
        assert!(result.should_allow());
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
}

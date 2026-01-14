//! HTTP request handler with classification and rule evaluation.
//!
//! Processes intercepted requests, applies classification, and decides
//! whether to block or forward.

use std::sync::Arc;

use http_body_util::{BodyExt, Full};
use hudsucker::{
    hyper::{Request, Response},
    Body, HttpContext, HttpHandler, RequestOrResponse,
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
}

impl std::fmt::Debug for HandlerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerConfig")
            .field("classifier", &"TieredClassifier")
            .field("rule_engine", &"RuleEngine")
            .field("notifications", &self.notifications.is_some())
            .field("on_block", &self.on_block.is_some())
            .field("on_allow", &self.on_allow.is_some())
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
    pub fn with_defaults() -> Self {
        Self::new(HandlerConfig {
            classifier: Arc::new(RwLock::new(TieredClassifier::keyword_only())),
            rule_engine: Arc::new(RuleEngine::with_defaults()),
            notifications: Some(Arc::new(NotificationManager::new())),
            on_block: None,
            on_allow: None,
        })
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
                tracing::debug!("No prompt extracted from {}{}", host, path);
                return RequestOrResponse::Request(Request::from_parts(
                    parts,
                    bytes_to_body(body_bytes),
                ));
            }
        };

        tracing::debug!(
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
        req: Request<Body>,
    ) -> RequestOrResponse {
        let host = match Self::extract_host(&req) {
            Some(h) => h,
            None => return RequestOrResponse::Request(req),
        };

        // Only intercept LLM domains
        if !is_llm_domain(&host) {
            return RequestOrResponse::Request(req);
        }

        tracing::debug!("Intercepting request to LLM domain: {}", host);

        self.handle_llm_request(&host, req).await
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        // Pass through responses unchanged
        res
    }
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
        };
        let debug = format!("{:?}", config);
        assert!(debug.contains("HandlerConfig"));
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

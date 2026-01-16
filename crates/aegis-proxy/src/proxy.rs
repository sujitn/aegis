//! MITM proxy server.
//!
//! Provides the main proxy server that intercepts HTTPS traffic to LLM services.

use std::net::SocketAddr;
use std::sync::Arc;

use hudsucker::rustls::crypto::aws_lc_rs::default_provider;
use hudsucker::Proxy;
use parking_lot::RwLock;
use tokio::sync::broadcast;

use aegis_core::classifier::TieredClassifier;
use aegis_core::notifications::NotificationManager;
use aegis_core::rule_engine::RuleEngine;

use crate::ca::CaManager;
use crate::error::{ProxyError, Result};
use crate::extractor::PromptInfo;
use crate::handler::{
    FilteringState, HandlerConfig, OnAllowCallback, OnBlockCallback, ProxyHandler,
};
use crate::DEFAULT_PROXY_PORT;

/// Proxy server configuration.
#[derive(Clone)]
pub struct ProxyConfig {
    /// Address to bind the proxy to.
    pub addr: SocketAddr,
    /// The CA manager for certificate generation.
    pub ca_manager: CaManager,
    /// The classifier for content analysis.
    pub classifier: Arc<RwLock<TieredClassifier>>,
    /// The rule engine for policy evaluation.
    pub rule_engine: Arc<RuleEngine>,
    /// Optional notification manager.
    pub notifications: Option<Arc<NotificationManager>>,
    /// Shared filtering state (controlled by ProfileProxyController).
    pub filtering_state: FilteringState,
}

impl std::fmt::Debug for ProxyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyConfig")
            .field("addr", &self.addr)
            .field("ca_manager", &self.ca_manager)
            .field("classifier", &"TieredClassifier")
            .field("rule_engine", &"RuleEngine")
            .field("notifications", &self.notifications.is_some())
            .field("filtering_state", &self.filtering_state)
            .finish()
    }
}

impl ProxyConfig {
    /// Creates a new configuration with default settings.
    ///
    /// Uses community rules for classification by default, which allows
    /// configurable patterns through the UI.
    pub fn new() -> Result<Self> {
        let ca_manager = CaManager::with_default_dir().map_err(ProxyError::Ca)?;

        Ok(Self {
            addr: SocketAddr::from(([127, 0, 0, 1], DEFAULT_PROXY_PORT)),
            ca_manager,
            // Use default classifier which includes community rules
            classifier: Arc::new(RwLock::new(TieredClassifier::with_defaults())),
            rule_engine: Arc::new(RuleEngine::with_defaults()),
            notifications: Some(Arc::new(NotificationManager::new())),
            filtering_state: FilteringState::new(),
        })
    }

    /// Creates a new configuration with the given filtering state.
    ///
    /// This allows external control of filtering (e.g., by ProfileProxyController).
    pub fn with_filtering_state(filtering_state: FilteringState) -> Result<Self> {
        let ca_manager = CaManager::with_default_dir().map_err(ProxyError::Ca)?;

        Ok(Self {
            addr: SocketAddr::from(([127, 0, 0, 1], DEFAULT_PROXY_PORT)),
            ca_manager,
            classifier: Arc::new(RwLock::new(TieredClassifier::with_defaults())),
            rule_engine: Arc::new(RuleEngine::with_defaults()),
            notifications: Some(Arc::new(NotificationManager::new())),
            filtering_state,
        })
    }

    /// Sets the filtering state.
    pub fn set_filtering_state(mut self, filtering_state: FilteringState) -> Self {
        self.filtering_state = filtering_state;
        self
    }

    /// Sets the listen address.
    pub fn with_addr(mut self, addr: SocketAddr) -> Self {
        self.addr = addr;
        self
    }

    /// Sets the port (uses 127.0.0.1 as host).
    pub fn with_port(mut self, port: u16) -> Self {
        self.addr = SocketAddr::from(([127, 0, 0, 1], port));
        self
    }

    /// Sets the CA manager.
    pub fn with_ca_manager(mut self, ca_manager: CaManager) -> Self {
        self.ca_manager = ca_manager;
        self
    }

    /// Sets the classifier.
    pub fn with_classifier(mut self, classifier: TieredClassifier) -> Self {
        self.classifier = Arc::new(RwLock::new(classifier));
        self
    }

    /// Sets the rule engine.
    pub fn with_rule_engine(mut self, rule_engine: RuleEngine) -> Self {
        self.rule_engine = Arc::new(rule_engine);
        self
    }

    /// Sets the notification manager.
    pub fn with_notifications(mut self, notifications: NotificationManager) -> Self {
        self.notifications = Some(Arc::new(notifications));
        self
    }

    /// Disables notifications.
    pub fn without_notifications(mut self) -> Self {
        self.notifications = None;
        self
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self::new().expect("Failed to create default proxy config")
    }
}

/// MITM proxy server for intercepting LLM traffic.
pub struct ProxyServer {
    config: ProxyConfig,
    on_block: Option<OnBlockCallback>,
    on_allow: Option<OnAllowCallback>,
}

impl ProxyServer {
    /// Creates a new proxy server with the given configuration.
    pub fn new(config: ProxyConfig) -> Result<Self> {
        // Ensure CA exists (will generate if missing)
        config.ca_manager.ensure_ca().map_err(ProxyError::Ca)?;

        Ok(Self {
            config,
            on_block: None,
            on_allow: None,
        })
    }

    /// Creates a proxy server with default configuration.
    pub fn with_defaults() -> Result<Self> {
        Self::new(ProxyConfig::new()?)
    }

    /// Sets the callback for blocked requests.
    pub fn on_block<F>(mut self, callback: F) -> Self
    where
        F: Fn(&PromptInfo, &aegis_core::rule_engine::RuleEngineResult) + Send + Sync + 'static,
    {
        self.on_block = Some(Arc::new(callback));
        self
    }

    /// Sets the callback for allowed requests.
    pub fn on_allow<F>(mut self, callback: F) -> Self
    where
        F: Fn(&PromptInfo, &aegis_core::rule_engine::RuleEngineResult) + Send + Sync + 'static,
    {
        self.on_allow = Some(Arc::new(callback));
        self
    }

    /// Returns the address the proxy is configured to listen on.
    pub fn addr(&self) -> SocketAddr {
        self.config.addr
    }

    /// Returns the CA certificate path for user installation.
    pub fn ca_cert_path(&self) -> std::path::PathBuf {
        self.config.ca_manager.cert_path()
    }

    /// Returns the CA certificate as DER bytes.
    pub fn ca_cert_der(&self) -> Result<Vec<u8>> {
        self.config
            .ca_manager
            .read_cert_der()
            .map_err(ProxyError::Ca)
    }

    /// Returns the filtering state.
    ///
    /// This can be used to control filtering externally (e.g., by ProfileProxyController).
    pub fn filtering_state(&self) -> &FilteringState {
        &self.config.filtering_state
    }

    /// Starts the proxy server.
    ///
    /// This will block until the server is shut down.
    pub async fn run(self) -> Result<()> {
        // Load CA authority
        let authority = self.config.ca_manager.ensure_ca().map_err(ProxyError::Ca)?;

        let handler_config = HandlerConfig {
            classifier: self.config.classifier.clone(),
            rule_engine: self.config.rule_engine.clone(),
            notifications: self.config.notifications.clone(),
            on_block: self.on_block.clone(),
            on_allow: self.on_allow.clone(),
            filtering_state: self.config.filtering_state.clone(),
        };

        let handler = ProxyHandler::new(handler_config);

        tracing::info!("Starting MITM proxy on {}", self.config.addr);
        tracing::info!("CA certificate: {:?}", self.ca_cert_path());

        let proxy = Proxy::builder()
            .with_addr(self.config.addr)
            .with_ca(authority)
            .with_rustls_connector(default_provider())
            .with_http_handler(handler.clone())
            .with_websocket_handler(handler)
            .build()
            .map_err(|e| ProxyError::Proxy(e.to_string()))?;

        // Run the proxy
        proxy
            .start()
            .await
            .map_err(|e| ProxyError::Proxy(e.to_string()))?;

        tracing::info!("Proxy server stopped");
        Ok(())
    }

    /// Starts the proxy server in the background.
    ///
    /// Returns a handle that can be used to stop the server.
    pub fn start(self) -> Result<ProxyHandle> {
        let (shutdown_tx, _) = broadcast::channel::<()>(1);
        let shutdown_tx_clone = shutdown_tx.clone();
        let addr = self.config.addr;

        // Load CA authority before spawning
        let authority = self.config.ca_manager.ensure_ca().map_err(ProxyError::Ca)?;

        let handler_config = HandlerConfig {
            classifier: self.config.classifier.clone(),
            rule_engine: self.config.rule_engine.clone(),
            notifications: self.config.notifications.clone(),
            on_block: self.on_block.clone(),
            on_allow: self.on_allow.clone(),
            filtering_state: self.config.filtering_state.clone(),
        };

        let config_addr = self.config.addr;

        let handle = tokio::spawn(async move {
            let handler = ProxyHandler::new(handler_config);

            let proxy = match Proxy::builder()
                .with_addr(config_addr)
                .with_ca(authority)
                .with_rustls_connector(default_provider())
                .with_http_handler(handler.clone())
                .with_websocket_handler(handler)
                .build()
            {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Failed to build proxy: {}", e);
                    return;
                }
            };

            let mut shutdown_rx = shutdown_tx.subscribe();

            tokio::select! {
                result = proxy.start() => {
                    if let Err(e) = result {
                        tracing::error!("Proxy error: {}", e);
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Proxy shutdown signal received");
                }
            };
        });

        Ok(ProxyHandle {
            shutdown_tx: shutdown_tx_clone,
            addr,
            handle,
        })
    }
}

/// Handle for controlling a running proxy server.
pub struct ProxyHandle {
    shutdown_tx: broadcast::Sender<()>,
    addr: SocketAddr,
    handle: tokio::task::JoinHandle<()>,
}

impl ProxyHandle {
    /// Returns the address the proxy is listening on.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Signals the proxy to shut down.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    /// Waits for the proxy to finish.
    pub async fn wait(self) {
        let _ = self.handle.await;
    }

    /// Shuts down the proxy and waits for it to finish.
    pub async fn stop(self) {
        self.shutdown();
        self.wait().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config() -> ProxyConfig {
        let temp_dir = TempDir::new().unwrap();
        let ca_manager = CaManager::new(temp_dir.path().join("ca"));

        ProxyConfig {
            addr: SocketAddr::from(([127, 0, 0, 1], 0)), // Random port
            ca_manager,
            classifier: Arc::new(RwLock::new(TieredClassifier::keyword_only())),
            rule_engine: Arc::new(RuleEngine::with_defaults()),
            notifications: None,
            filtering_state: FilteringState::new(),
        }
    }

    #[test]
    fn proxy_config_with_port() {
        let config = test_config().with_port(8888);
        assert_eq!(config.addr.port(), 8888);
    }

    #[test]
    fn proxy_config_with_addr() {
        let addr = SocketAddr::from(([0, 0, 0, 0], 9999));
        let config = test_config().with_addr(addr);
        assert_eq!(config.addr, addr);
    }

    #[test]
    fn proxy_config_without_notifications() {
        let config = test_config().without_notifications();
        assert!(config.notifications.is_none());
    }

    #[test]
    fn proxy_server_new() {
        let config = test_config();
        let server = ProxyServer::new(config);
        assert!(server.is_ok());
    }

    #[test]
    fn proxy_server_ca_paths() {
        let config = test_config();
        let server = ProxyServer::new(config).unwrap();

        let cert_path = server.ca_cert_path();
        assert!(cert_path.to_string_lossy().contains("aegis-ca.crt"));

        let der = server.ca_cert_der();
        assert!(der.is_ok());
    }

    #[tokio::test]
    async fn proxy_handle_shutdown() {
        let config = test_config();
        let server = ProxyServer::new(config).unwrap();

        let handle = server.start().unwrap();

        // Give it a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Stop it
        handle.stop().await;
    }
}

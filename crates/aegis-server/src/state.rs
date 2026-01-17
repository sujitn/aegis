//! Application state for the API server.

use std::sync::{Arc, RwLock};

use aegis_core::auth::AuthManager;
use aegis_core::classifier::{SentimentAnalyzer, SentimentConfig, TieredClassifier};
use aegis_core::profile::ProfileManager;
use aegis_core::rule_engine::RuleEngine;
use aegis_storage::Database;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Database connection.
    pub db: Arc<Database>,
    /// Authentication manager.
    pub auth: Arc<AuthManager>,
    /// Content classifier (requires mutable access).
    pub classifier: Arc<RwLock<TieredClassifier>>,
    /// Rule engine for evaluation.
    pub rules: Arc<RwLock<RuleEngine>>,
    /// User profiles for per-user rules.
    pub profiles: Arc<RwLock<ProfileManager>>,
    /// Sentiment analyzer for emotional content flagging.
    pub sentiment_analyzer: Arc<RwLock<SentimentAnalyzer>>,
}

impl AppState {
    /// Creates a new application state with the given database.
    pub fn new(db: Database) -> Self {
        Self {
            db: Arc::new(db),
            auth: Arc::new(AuthManager::new()),
            classifier: Arc::new(RwLock::new(TieredClassifier::keyword_only())),
            rules: Arc::new(RwLock::new(RuleEngine::with_defaults())),
            profiles: Arc::new(RwLock::new(ProfileManager::new())),
            sentiment_analyzer: Arc::new(RwLock::new(SentimentAnalyzer::new(
                SentimentConfig::default(),
            ))),
        }
    }

    /// Creates application state with default in-memory database.
    pub fn in_memory() -> Self {
        Self::new(Database::in_memory().expect("Failed to create in-memory database"))
    }

    /// Creates application state with custom components.
    pub fn with_components(
        db: Database,
        auth: AuthManager,
        classifier: TieredClassifier,
        rules: RuleEngine,
        profiles: ProfileManager,
    ) -> Self {
        Self {
            db: Arc::new(db),
            auth: Arc::new(auth),
            classifier: Arc::new(RwLock::new(classifier)),
            rules: Arc::new(RwLock::new(rules)),
            profiles: Arc::new(RwLock::new(profiles)),
            sentiment_analyzer: Arc::new(RwLock::new(SentimentAnalyzer::new(
                SentimentConfig::default(),
            ))),
        }
    }
}

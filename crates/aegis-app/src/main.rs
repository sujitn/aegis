//! Aegis - AI safety platform for filtering LLM interactions.
//!
//! This is the main binary that integrates all Aegis components.

use aegis_core::{auth::AuthManager, classifier::KeywordClassifier, rule_engine::RuleEngine};
use aegis_server::{Server, ServerConfig};
use aegis_storage::db::Database;
use aegis_tray::SystemTray;
use aegis_ui::settings::SettingsUi;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Aegis...");

    // Initialize all components
    let _classifier = KeywordClassifier::new();
    let _rules = RuleEngine::new();
    let _auth = AuthManager::new();
    let _db = Database::new();
    let _server = Server::new(ServerConfig::default()).await?;
    let _ui = SettingsUi::new();
    let (_tray, _tray_rx) = SystemTray::new()?;

    tracing::info!("Aegis initialized successfully");

    Ok(())
}

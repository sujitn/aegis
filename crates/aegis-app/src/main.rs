//! Aegis - AI safety platform for filtering LLM interactions.
//!
//! This is the main binary that integrates all Aegis components.

use aegis_core::{auth::Auth, classifier::Classifier, rules::RuleEngine};
use aegis_server::api::Server;
use aegis_storage::db::Database;
use aegis_tray::tray::SystemTray;
use aegis_ui::settings::SettingsUi;

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Aegis...");

    // Initialize all components (placeholders for now)
    let _classifier = Classifier::new();
    let _rules = RuleEngine::new();
    let _auth = Auth::new();
    let _db = Database::new();
    let _server = Server::new();
    let _ui = SettingsUi::new();
    let _tray = SystemTray::new();

    tracing::info!("Aegis initialized successfully");

    Ok(())
}

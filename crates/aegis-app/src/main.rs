//! Aegis - AI safety platform for filtering LLM interactions.
//!
//! This is the main binary that runs the full Aegis application:
//! - HTTP API server (for browser extension)
//! - MITM Proxy server (for system-wide protection)
//! - Parent Dashboard GUI (for management)

use aegis_proxy::{ProxyConfig, ProxyServer};
use aegis_server::{Server, ServerConfig};
use aegis_storage::Database;
use aegis_ui::run_dashboard;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("aegis=info".parse().unwrap()),
        )
        .init();

    tracing::info!("Starting Aegis...");

    // Open the database (creates if doesn't exist)
    let db = Database::new().map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
    tracing::info!("Database opened at {:?}", Database::default_db_path()?);

    // Clone for servers (servers need their own handles)
    let server_db = db.clone();

    // Start HTTP API server in background (for browser extension)
    let server_config = ServerConfig::default();
    let server_addr = format!("{}:{}", server_config.host, server_config.port);

    tokio::spawn(async move {
        tracing::info!("Starting API server on {}", server_addr);
        match Server::with_database(ServerConfig::default(), server_db) {
            Ok(server) => {
                if let Err(e) = server.run().await {
                    tracing::error!("API server error: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to create API server: {}", e);
            }
        }
    });

    // Start MITM proxy server in background (for system-wide protection)
    tokio::spawn(async move {
        match ProxyConfig::new() {
            Ok(config) => {
                let proxy_addr = config.addr;
                match ProxyServer::new(config) {
                    Ok(proxy) => {
                        let ca_cert_path = proxy.ca_cert_path();
                        tracing::info!("Starting MITM proxy on {}", proxy_addr);
                        tracing::info!("CA certificate: {:?}", ca_cert_path);

                        if let Err(e) = proxy.run().await {
                            tracing::error!("Proxy server error: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to create proxy server: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to create proxy config: {}", e);
            }
        }
    });

    // Give servers a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    tracing::info!("Launching Dashboard UI...");

    // Run the dashboard UI (blocking - this is the main event loop)
    // The dashboard handles:
    // - First-run setup wizard (if not configured)
    // - Login screen (if configured)
    // - Full dashboard after login
    run_dashboard(db).map_err(|e| anyhow::anyhow!("UI error: {}", e))?;

    tracing::info!("Aegis shutting down");
    Ok(())
}

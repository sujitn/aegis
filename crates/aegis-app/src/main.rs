//! Aegis - AI safety platform for filtering LLM interactions.
//!
//! This is the main binary that runs the full Aegis application:
//! - HTTP API server (for browser extension)
//! - MITM Proxy server (for system-wide protection)
//! - System tray (primary interface)
//! - Parent Dashboard GUI (opens on demand)

// Hide console window on Windows (logs go to file instead)
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::panic;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use aegis_core::content_rules::ContentRuleSet;
use aegis_core::profile::{ProfileManager, ProxyMode, UserProfile};
use aegis_core::profile_proxy::{ProfileProxyConfig, ProfileProxyController, ProxyAction};
use aegis_core::protection::ProtectionManager;
use aegis_core::time_rules::TimeRuleSet;
use aegis_proxy::{FilteringState, ProxyConfig, ProxyServer};
use aegis_server::{Server, ServerConfig};
use aegis_storage::Database;
use aegis_tray::{MenuAction, SystemTray, TrayConfig, TrayEvent, TrayStatus};
use aegis_ui::run_dashboard;
use clap::Parser;
use directories::ProjectDirs;
use muda::MenuEvent;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tray_icon::TrayIconEvent;

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
};

/// Aegis - AI safety platform for filtering LLM interactions
#[derive(Parser, Debug)]
#[command(name = "aegis", version, about)]
struct Args {
    /// Skip tray icon, open dashboard directly
    #[arg(long)]
    no_tray: bool,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,

    /// Set log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Start with dashboard visible (for first-run or debugging)
    #[arg(long)]
    show_dashboard: bool,

    /// Start minimized to tray (used by autostart)
    #[arg(long)]
    minimized: bool,
}

/// Get the logs directory path.
fn logs_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "aegis", "Aegis").map(|dirs| dirs.data_dir().join("logs"))
}

/// Initialize logging with file rotation.
fn init_logging(args: &Args) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let log_level = if args.debug { "debug" } else { &args.log_level };

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("aegis={},warn", log_level)));

    // Try to set up file logging
    if let Some(log_dir) = logs_dir() {
        // Create logs directory if it doesn't exist
        if std::fs::create_dir_all(&log_dir).is_ok() {
            // Create rolling file appender (rotates daily, keeps files)
            let file_appender = RollingFileAppender::builder()
                .rotation(Rotation::DAILY)
                .max_log_files(5)
                .filename_prefix("aegis")
                .filename_suffix("log")
                .build(&log_dir)
                .ok();

            if let Some(appender) = file_appender {
                let (non_blocking, guard) = tracing_appender::non_blocking(appender);

                // In debug mode, also log to console
                if args.debug || args.no_tray {
                    tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt::layer().with_writer(std::io::stdout))
                        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
                        .init();
                } else {
                    // Release mode: file only (no console since we're windowless)
                    tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
                        .init();
                }

                tracing::info!("Logging to {:?}", log_dir);
                return Some(guard);
            }
        }
    }

    // Fallback: console logging only
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    tracing::warn!("File logging unavailable, using console only");
    None
}

/// Check if this is the first run (setup not completed).
fn is_first_run(db: &Database) -> bool {
    db.get_config("setup_completed").ok().flatten().is_none()
}

/// Drain any stale events from global event receivers.
/// This prevents old events from affecting newly created tray instances.
fn drain_stale_events() {
    // Drain menu events
    let menu_receiver = MenuEvent::receiver();
    while menu_receiver.try_recv().is_ok() {}

    // Drain tray icon events
    let tray_receiver = TrayIconEvent::receiver();
    while tray_receiver.try_recv().is_ok() {}

    tracing::debug!("Drained stale events from global receivers");
}

/// Pump Windows messages to allow tray icon to receive events.
/// On Windows, the tray icon requires the message loop to be pumped.
#[cfg(target_os = "windows")]
fn pump_windows_messages() {
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        // Process all pending messages without blocking
        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// No-op on non-Windows platforms.
#[cfg(not(target_os = "windows"))]
fn pump_windows_messages() {
    // No-op on non-Windows
}

/// Load profiles from database and convert to ProfileManager.
fn load_profiles_from_db(db: &Database) -> ProfileManager {
    let mut manager = ProfileManager::new();

    match db.get_all_profiles() {
        Ok(profiles) => {
            for profile in profiles {
                // Convert storage Profile to core UserProfile
                let time_rules: TimeRuleSet = serde_json::from_value(profile.time_rules.clone())
                    .unwrap_or_else(|_| TimeRuleSet::new());
                let content_rules: ContentRuleSet =
                    serde_json::from_value(profile.content_rules.clone())
                        .unwrap_or_else(|_| ContentRuleSet::new());

                // Determine proxy mode based on profile name heuristic
                // Profiles with "parent" in the name disable filtering; others enable it
                // This is because profiles in the database are typically child profiles
                let is_parent = profile.name.to_lowercase().contains("parent");
                let proxy_mode = if is_parent {
                    ProxyMode::Disabled
                } else {
                    ProxyMode::Enabled
                };

                let mut user_profile = UserProfile::new(
                    format!("profile_{}", profile.id),
                    &profile.name,
                    profile.os_username.clone(),
                    time_rules,
                    content_rules,
                );
                user_profile.enabled = profile.enabled;
                user_profile.proxy_mode = proxy_mode;

                manager.add_profile(user_profile);
                tracing::debug!(
                    "Loaded profile: {} (os_username: {:?}, proxy_mode: {:?})",
                    profile.name,
                    profile.os_username,
                    proxy_mode
                );
            }
            tracing::info!("Loaded {} profiles from database", manager.profile_count());
        }
        Err(e) => {
            tracing::warn!("Failed to load profiles from database: {}", e);
        }
    }

    manager
}

/// Start the background servers (API and Proxy) with profile-aware filtering.
async fn start_servers(db: Database) {
    let server_db = db.clone();
    let profile_db = db.clone();
    let proxy_db = Arc::new(db.clone());

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

    // Load profiles and create profile-aware filtering
    let profiles = load_profiles_from_db(&profile_db);
    let protection = ProtectionManager::new();
    let filtering_state = FilteringState::new();

    // Create profile proxy controller with callback to control filtering
    let filtering_state_clone = filtering_state.clone();
    let controller =
        ProfileProxyController::new(profiles, protection, ProfileProxyConfig::default()).on_switch(
            move |event| {
                tracing::info!(
                    "Profile switch: {} -> {} (action: {})",
                    event.previous_profile.as_deref().unwrap_or("none"),
                    event.new_profile.as_deref().unwrap_or("none"),
                    event.proxy_action
                );

                // Update filtering state based on proxy action
                match event.proxy_action {
                    ProxyAction::Enabled => {
                        filtering_state_clone.enable();
                        filtering_state_clone.set_profile(event.new_profile.clone());
                    }
                    ProxyAction::Disabled | ProxyAction::Passthrough => {
                        filtering_state_clone.disable();
                        filtering_state_clone.set_profile(event.new_profile.clone());
                    }
                    ProxyAction::NoChange => {}
                }
            },
        );

    // Initialize controller to detect current user and set initial filtering state
    if let Some(event) = controller.initialize() {
        tracing::info!(
            "Initial profile: {:?} (proxy action: {})",
            event.new_profile,
            event.proxy_action
        );
    }

    // Start monitoring for user changes
    controller.start_monitoring();
    let poll_interval = controller.poll_interval();

    // Spawn profile monitoring task
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(poll_interval).await;
            if let Some(event) = controller.poll_once() {
                tracing::info!(
                    "Profile change detected: {} -> {}",
                    event.previous_profile.as_deref().unwrap_or("none"),
                    event.new_profile.as_deref().unwrap_or("none")
                );
            }
        }
    });

    // Start MITM proxy server in background (for system-wide protection)
    // Use the shared filtering state so ProfileProxyController can control filtering
    // Also pass the database for event logging (live stats)
    tokio::spawn(async move {
        match ProxyConfig::with_filtering_state(filtering_state) {
            Ok(config) => {
                let config = config.with_database(proxy_db);
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
}

/// Run the application with tray icon as primary interface.
fn run_with_tray(db: Database, show_dashboard: bool) -> anyhow::Result<()> {
    // Set up panic hook to log panics to file
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        tracing::error!("PANIC: {}", panic_info);
        default_hook(panic_info);
    }));

    let running = Arc::new(AtomicBool::new(true));

    // Track if we should open dashboard
    let mut should_open_dashboard = show_dashboard;
    let mut tray_status = TrayStatus::Protected;

    // Main loop - reinitialize tray after each dashboard session
    while running.load(Ordering::SeqCst) {
        // Drain any stale events from previous tray instance
        drain_stale_events();

        // Small delay to let resources clean up
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Create and initialize the system tray
        tracing::debug!("Creating new system tray...");
        let (mut tray, tray_rx) =
            SystemTray::with_config(TrayConfig::new().with_initial_status(tray_status))?;

        tracing::debug!("Initializing system tray...");
        tray.init()?;
        tracing::info!("System tray initialized");

        // If we should open dashboard immediately, do it and skip tray loop
        if should_open_dashboard {
            should_open_dashboard = false;
            tracing::debug!("Shutting down tray for dashboard...");
            tray.shutdown();

            // Drain events after shutdown
            drain_stale_events();

            tracing::info!("Opening dashboard...");
            let db_clone = db.clone();
            if let Err(e) = run_dashboard(db_clone) {
                tracing::error!("Dashboard error: {}", e);
            }
            tracing::info!("Dashboard closed, returning to tray");
            continue;
        }

        // Tray event loop
        tracing::debug!("Entering tray event loop...");
        loop {
            // Pump Windows messages to allow tray to receive events
            pump_windows_messages();

            // Poll tray events
            let events = tray.poll_events();

            for event in events {
                match event {
                    TrayEvent::MenuAction(action) => {
                        tracing::debug!("Tray menu action: {:?}", action);
                        match action {
                            MenuAction::Dashboard | MenuAction::Settings => {
                                should_open_dashboard = true;
                            }
                            MenuAction::Logs => {
                                // Open logs folder
                                if let Some(log_dir) = logs_dir() {
                                    let _ = open::that(&log_dir);
                                }
                            }
                            MenuAction::Pause => {
                                tray_status = TrayStatus::Paused;
                                let _ = tray.set_status(tray_status);
                            }
                            MenuAction::Resume => {
                                tray_status = TrayStatus::Protected;
                                let _ = tray.set_status(tray_status);
                            }
                            MenuAction::Quit => {
                                tracing::info!("Quit requested from tray");
                                running.store(false, Ordering::SeqCst);
                            }
                        }
                    }
                    TrayEvent::DoubleClick => {
                        should_open_dashboard = true;
                    }
                    TrayEvent::StatusChanged(status) => {
                        tracing::info!("Protection status changed: {:?}", status);
                        tray_status = status;
                    }
                }
            }

            // Check for events from channel
            while let Ok(event) = tray_rx.try_recv() {
                if let TrayEvent::MenuAction(MenuAction::Quit) = event {
                    running.store(false, Ordering::SeqCst);
                }
            }

            // Break inner loop if we need to open dashboard or quit
            if should_open_dashboard || !running.load(Ordering::SeqCst) {
                tracing::debug!(
                    "Breaking tray loop: dashboard={}, running={}",
                    should_open_dashboard,
                    running.load(Ordering::SeqCst)
                );
                break;
            }

            // Small sleep to prevent busy loop
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // Shutdown tray before opening dashboard or quitting
        tracing::debug!("Shutting down tray...");
        tray.shutdown();

        // Drain events after shutdown
        drain_stale_events();

        // Open dashboard if requested
        if should_open_dashboard && running.load(Ordering::SeqCst) {
            should_open_dashboard = false;
            tracing::info!("Opening dashboard...");

            let db_clone = db.clone();
            if let Err(e) = run_dashboard(db_clone) {
                tracing::error!("Dashboard error: {}", e);
            }
            tracing::info!("Dashboard closed, returning to tray");
        }
    }

    tracing::debug!("Exiting run_with_tray");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging (keep guard alive for the duration of the program)
    let _log_guard = init_logging(&args);

    tracing::info!("Starting Aegis...");
    tracing::info!("Args: {:?}", args);

    // Open the database (creates if doesn't exist)
    let db = Database::new().map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
    tracing::info!("Database opened at {:?}", Database::default_db_path()?);

    // Start background servers
    start_servers(db.clone()).await;

    // Determine startup mode
    let first_run = is_first_run(&db);
    // Show dashboard if:
    // - First run (setup wizard needed) - always, even if --minimized is set
    // - Explicitly requested via --show-dashboard
    // When --minimized is set (autostart mode), start silently unless first_run
    let show_dashboard = first_run || args.show_dashboard;

    if args.no_tray {
        // No tray mode: just run dashboard directly
        tracing::info!("Running in no-tray mode (dashboard only)");
        run_dashboard(db).map_err(|e| anyhow::anyhow!("UI error: {}", e))?;
    } else {
        // Normal mode: tray icon with dashboard on demand
        tracing::info!(
            "Running in tray mode (first_run={}, show_dashboard={}, minimized={})",
            first_run,
            show_dashboard,
            args.minimized
        );
        run_with_tray(db, show_dashboard)?;
    }

    tracing::info!("Aegis shutting down");
    Ok(())
}

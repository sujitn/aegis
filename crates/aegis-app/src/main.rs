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

use aegis_core::classifier::{SentimentConfig, SentimentFlag};
use aegis_core::content_rules::ContentRuleSet;
use aegis_core::profile::{ProfileManager, ProxyMode, UserProfile};
use aegis_core::profile_proxy::{ProfileProxyConfig, ProfileProxyController, ProxyAction};
use aegis_core::protection::ProtectionManager;
use aegis_core::rule_engine::RuleEngine;
use aegis_core::time_rules::TimeRuleSet;
use aegis_proxy::{FilteringState, ProxyConfig, ProxyServer};
use aegis_server::{AppState as ServerAppState, Server, ServerConfig};
use aegis_storage::models::ProfileSentimentConfig;
use aegis_storage::Database;
use aegis_tray::{MenuAction, SystemTray, TrayConfig, TrayEvent, TrayStatus};
use aegis_ui::run_dashboard_with_filtering;
use clap::Parser;
use directories::ProjectDirs;
use muda::MenuEvent;
use std::collections::HashSet;
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

    /// Run dashboard only (internal use - spawned from main process)
    #[arg(long, hide = true)]
    dashboard_only: bool,
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
/// Returns the shared FilteringState for use by the UI.
async fn start_servers(db: Database) -> FilteringState {
    let server_db = db.clone();
    let profile_db = db.clone();
    let rules_db = db.clone();
    let proxy_db = Arc::new(db.clone());

    // Load profiles and create profile-aware filtering
    let profiles = load_profiles_from_db(&profile_db);
    let protection = ProtectionManager::new();

    // Load initial rules from the first enabled profile (if any)
    let initial_rule_engine = load_initial_rules(&rules_db);

    // Create FilteringState with StateCache for database-backed protection state (F032)
    let filtering_state =
        FilteringState::with_rule_engine_and_cache(initial_rule_engine, proxy_db.clone());

    // Start HTTP API server in background (for browser extension)
    // Pass the FilteringState so the reload endpoint can update it
    let server_config = ServerConfig::default();
    let server_addr = format!("{}:{}", server_config.host, server_config.port);
    let server_filtering_state = filtering_state.clone();

    tokio::spawn(async move {
        tracing::info!("Starting API server on {}", server_addr);
        let app_state = ServerAppState::with_filtering_state(server_db, server_filtering_state);
        match Server::with_state(ServerConfig::default(), app_state) {
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

    // Create profile proxy controller with callback to control filtering and update rules
    let filtering_state_clone = filtering_state.clone();
    let switch_db = rules_db.clone();
    let controller = ProfileProxyController::new(
        profiles,
        protection,
        ProfileProxyConfig::default(),
    )
    .on_switch(move |event| {
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

                // Load rules for the new profile
                if let Some(ref profile_name) = event.new_profile {
                    load_profile_rules_by_name(&switch_db, profile_name, &filtering_state_clone);
                }
            }
            ProxyAction::Disabled | ProxyAction::Passthrough => {
                filtering_state_clone.disable();
                filtering_state_clone.set_profile(event.new_profile.clone());
            }
            ProxyAction::NoChange => {}
        }
    });

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

    // Clone filtering_state for proxy and for return
    let proxy_filtering_state = filtering_state.clone();
    let return_filtering_state = filtering_state.clone();

    // Start MITM proxy server in background (for system-wide protection)
    // Use the shared filtering state so ProfileProxyController can control filtering
    // Also pass the database for event logging (live stats)
    tokio::spawn(async move {
        match ProxyConfig::with_filtering_state(proxy_filtering_state) {
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

    return_filtering_state
}

/// Loads the initial rule engine from the first enabled profile.
fn load_initial_rules(db: &Database) -> RuleEngine {
    match db.get_all_profiles() {
        Ok(profiles) => {
            // Find first enabled profile
            if let Some(profile) = profiles.iter().find(|p| p.enabled) {
                let time_rules: TimeRuleSet =
                    serde_json::from_value(profile.time_rules.clone()).unwrap_or_default();
                let content_rules: ContentRuleSet =
                    serde_json::from_value(profile.content_rules.clone()).unwrap_or_default();

                tracing::info!(
                    "Loaded initial rules from profile '{}': {} time rules, {} content rules",
                    profile.name,
                    time_rules.rules.len(),
                    content_rules.rules.len()
                );

                return RuleEngine {
                    time_rules,
                    content_rules,
                };
            }
        }
        Err(e) => {
            tracing::warn!("Failed to load profiles for initial rules: {}", e);
        }
    }

    tracing::info!("Using default rules (no enabled profiles found)");
    RuleEngine::with_defaults()
}

/// Loads rules for a specific profile by name and updates the filtering state.
/// Also sets the profile ID and enables sentiment analysis based on the profile's config.
fn load_profile_rules_by_name(db: &Database, profile_name: &str, filtering_state: &FilteringState) {
    match db.get_all_profiles() {
        Ok(profiles) => {
            if let Some(profile) = profiles.iter().find(|p| p.name == profile_name) {
                let time_rules: TimeRuleSet =
                    serde_json::from_value(profile.time_rules.clone()).unwrap_or_default();
                let content_rules: ContentRuleSet =
                    serde_json::from_value(profile.content_rules.clone()).unwrap_or_default();

                tracing::info!(
                    "Loading rules for profile '{}' (id={}): {} time rules, {} content rules",
                    profile.name,
                    profile.id,
                    time_rules.rules.len(),
                    content_rules.rules.len()
                );

                // Set profile ID for sentiment flagging
                filtering_state.set_profile_with_id(Some(profile.name.clone()), Some(profile.id));

                // Update rules
                filtering_state.update_rules(time_rules, content_rules);

                // Configure sentiment analysis based on profile settings
                configure_sentiment_analysis(filtering_state, &profile.sentiment_config);
            } else {
                tracing::warn!("Profile '{}' not found in database", profile_name);
            }
        }
        Err(e) => {
            tracing::warn!("Failed to load profile '{}' rules: {}", profile_name, e);
        }
    }
}

/// Configure sentiment analysis based on profile settings.
fn configure_sentiment_analysis(filtering_state: &FilteringState, config: &ProfileSentimentConfig) {
    if config.enabled {
        // Build enabled flags set from the profile config
        let mut enabled_flags = HashSet::new();
        if config.detect_distress {
            enabled_flags.insert(SentimentFlag::Distress);
        }
        if config.detect_crisis {
            enabled_flags.insert(SentimentFlag::CrisisIndicator);
        }
        if config.detect_bullying {
            enabled_flags.insert(SentimentFlag::Bullying);
        }
        if config.detect_negative {
            enabled_flags.insert(SentimentFlag::NegativeSentiment);
        }

        let sentiment_config = SentimentConfig {
            enabled: true,
            threshold: config.sensitivity,
            enabled_flags,
            notify_on_flag: true,
        };

        filtering_state.enable_sentiment_analysis(sentiment_config);
        tracing::info!(
            "Sentiment analysis enabled (sensitivity={}, distress={}, crisis={}, bullying={}, negative={})",
            config.sensitivity,
            config.detect_distress,
            config.detect_crisis,
            config.detect_bullying,
            config.detect_negative
        );
    } else {
        filtering_state.disable_sentiment_analysis();
        tracing::info!("Sentiment analysis disabled for profile");
    }
}

/// Spawn the dashboard as a separate process.
/// This allows the dashboard to exit without killing the main process.
/// Returns the Child process handle for tracking.
fn spawn_dashboard_process() -> anyhow::Result<std::process::Child> {
    let exe_path = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Failed to get current exe path: {}", e))?;

    tracing::info!("Spawning dashboard subprocess: {:?}", exe_path);

    let child = std::process::Command::new(&exe_path)
        .arg("--dashboard-only")
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn dashboard process: {}", e))?;

    tracing::debug!("Dashboard subprocess started, PID: {}", child.id());

    Ok(child)
}

/// Run the application with tray icon as primary interface.
#[allow(unused_assignments)]
fn run_with_tray(
    _db: Database,
    show_dashboard: bool,
    filtering_state: FilteringState,
) -> anyhow::Result<()> {
    // Set up panic hook to log panics to file
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        tracing::error!("PANIC: {}", panic_info);
        default_hook(panic_info);
    }));

    let running = Arc::new(AtomicBool::new(true));

    // Track dashboard subprocess
    let mut dashboard_process: Option<std::process::Child> = None;
    let mut tray_status = TrayStatus::Protected;

    // Drain any stale events
    drain_stale_events();

    // Create and initialize the system tray once
    tracing::debug!("Creating system tray...");
    let (mut tray, tray_rx) =
        SystemTray::with_config(TrayConfig::new().with_initial_status(tray_status))?;

    tracing::debug!("Initializing system tray...");
    tray.init()?;
    tracing::info!("System tray initialized");

    // Open dashboard immediately if requested (e.g., first run)
    if show_dashboard {
        tracing::info!("Opening dashboard (subprocess) on startup...");
        match spawn_dashboard_process() {
            Ok(child) => {
                dashboard_process = Some(child);
            }
            Err(e) => {
                tracing::error!("Failed to spawn dashboard: {}", e);
            }
        }
    }

    // Main event loop - tray stays alive the whole time
    tracing::debug!("Entering main event loop...");
    while running.load(Ordering::SeqCst) {
        // Pump Windows messages to allow tray to receive events
        pump_windows_messages();

        // Check if dashboard process has exited
        if let Some(ref mut child) = dashboard_process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    tracing::info!("Dashboard subprocess exited with status: {:?}", status);
                    dashboard_process = None;
                }
                Ok(None) => {
                    // Still running - don't log to avoid spam
                }
                Err(e) => {
                    tracing::error!("Error checking dashboard process: {}", e);
                    dashboard_process = None;
                }
            }
        } else {
            // No dashboard process running - tray menu should allow opening
        }

        // Poll tray events
        let events = tray.poll_events();

        for event in events {
            match event {
                TrayEvent::MenuAction(action) => {
                    tracing::debug!("Tray menu action: {:?}", action);
                    match action {
                        MenuAction::Dashboard | MenuAction::Settings => {
                            // Only open if not already open
                            if dashboard_process.is_none() {
                                tracing::info!("Opening dashboard (subprocess)...");
                                match spawn_dashboard_process() {
                                    Ok(child) => {
                                        tracing::info!(
                                            "Dashboard subprocess spawned, PID: {}",
                                            child.id()
                                        );
                                        dashboard_process = Some(child);
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to spawn dashboard: {}", e);
                                    }
                                }
                            } else {
                                tracing::warn!("Dashboard request ignored - subprocess still tracked as running");
                            }
                        }
                        MenuAction::Logs => {
                            // Open logs folder
                            if let Some(log_dir) = logs_dir() {
                                tracing::info!("Opening logs folder: {:?}", log_dir);
                                if let Err(e) = open::that(&log_dir) {
                                    tracing::error!("Failed to open logs folder: {}", e);
                                }
                            } else {
                                tracing::error!("Logs directory not found");
                            }
                        }
                        MenuAction::Pause => {
                            tracing::info!("Pausing filtering...");
                            filtering_state.disable();
                            tray_status = TrayStatus::Paused;
                            let _ = tray.set_status(tray_status);
                        }
                        MenuAction::Resume => {
                            tracing::info!("Resuming filtering...");
                            filtering_state.enable();
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
                    // Only open if not already open
                    if dashboard_process.is_none() {
                        tracing::info!("Opening dashboard (subprocess) via double-click...");
                        match spawn_dashboard_process() {
                            Ok(child) => {
                                dashboard_process = Some(child);
                            }
                            Err(e) => {
                                tracing::error!("Failed to spawn dashboard: {}", e);
                            }
                        }
                    }
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

        // Small sleep to prevent busy loop
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Kill dashboard if still running when we quit
    if let Some(mut child) = dashboard_process {
        tracing::info!("Terminating dashboard subprocess...");
        let _ = child.kill();
    }

    // Shutdown tray
    tracing::debug!("Shutting down tray...");
    tray.shutdown();
    drain_stale_events();

    tracing::debug!("Exiting run_with_tray");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging (keep guard alive for the duration of the program)
    let _log_guard = init_logging(&args);

    tracing::info!("Starting Aegis...");
    tracing::info!("Args: {:?}", args);

    // Open the database (creates if doesn't exist)
    let db = Database::new().map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
    tracing::info!("Database opened at {:?}", Database::default_db_path()?);

    // Dashboard-only mode: just run the dashboard UI (spawned from main process)
    if args.dashboard_only {
        tracing::info!("Running in dashboard-only mode (subprocess)");
        run_dashboard_with_filtering(db, None).map_err(|e| anyhow::anyhow!("UI error: {}", e))?;
        tracing::info!("Dashboard subprocess exiting");
        return Ok(());
    }

    // Create a tokio runtime for background servers
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create tokio runtime: {}", e))?;

    // Start background servers and get the shared filtering state
    let filtering_state = runtime.block_on(start_servers(db.clone()));

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
        run_dashboard_with_filtering(db, Some(filtering_state))
            .map_err(|e| anyhow::anyhow!("UI error: {}", e))?;
    } else {
        // Normal mode: tray icon with dashboard on demand
        tracing::info!(
            "Running in tray mode (first_run={}, show_dashboard={}, minimized={})",
            first_run,
            show_dashboard,
            args.minimized
        );
        run_with_tray(db, show_dashboard, filtering_state)?;
    }

    tracing::info!("Aegis shutting down");
    Ok(())
}

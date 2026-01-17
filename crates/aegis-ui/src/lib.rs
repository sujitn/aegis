//! Aegis UI - Parent Dashboard GUI (Dioxus-based).
//!
//! This crate provides the parent dashboard for the Aegis platform.
//! It includes:
//!
//! - Password-protected access with session timeout
//! - Dashboard with summary statistics and quick actions
//! - Profile management (create, edit, delete)
//! - Rules configuration (time rules, content rules)
//! - Activity logs with filtering and export
//! - Settings (password change, mode selection)
//!
//! # Usage
//!
//! ```no_run
//! use aegis_ui::run_dashboard;
//! use aegis_storage::Database;
//!
//! // Create database
//! let db = Database::new().expect("Failed to open database");
//!
//! // Run the dashboard
//! run_dashboard(db).expect("Failed to run dashboard");
//! ```

use std::sync::Mutex;

use aegis_proxy::FilteringState;
use dioxus::prelude::*;

mod components;
pub mod error;
pub mod state;
pub mod views;

pub use error::{Result, UiError};
pub use state::{AppState, InterceptionMode, ProtectionStatus, View};

/// CSS styles as a static string
const STYLES: &str = include_str!("../assets/styles.css");

/// Global initial state holder (set before launch, consumed at startup).
/// Uses Mutex<Option<>> so it can be replaced on each dashboard launch.
static INITIAL_STATE: Mutex<Option<AppState>> = Mutex::new(None);

/// Runs the parent dashboard application.
///
/// This is the main entry point for the GUI application.
pub fn run_dashboard(db: aegis_storage::Database) -> Result<()> {
    run_dashboard_with_filtering(db, None)
}

/// Runs the parent dashboard application with an optional filtering state.
///
/// If `filtering_state` is provided, rule changes made in the UI will be
/// immediately applied to the running proxy. This also indicates we're running
/// in-process (not as a subprocess), so we use CloseWindow behavior.
///
/// If `filtering_state` is None, we're running as a subprocess and should
/// exit when the window closes (LastWindowExitsApp behavior).
///
/// IMPORTANT: This function must be called from the main thread on Windows,
/// as the window event loop requires it.
pub fn run_dashboard_with_filtering(
    db: aegis_storage::Database,
    filtering_state: Option<FilteringState>,
) -> Result<()> {
    tracing::debug!("run_dashboard_with_filtering: starting");

    // Determine close behavior based on whether we have filtering_state
    // - With filtering_state: running in-process (--no-tray mode), use CloseWindow
    // - Without: running as subprocess, use LastWindowExitsApp so process exits
    let is_subprocess = filtering_state.is_none();
    let close_behaviour = if is_subprocess {
        tracing::debug!("Subprocess mode: using LastWindowExitsApp");
        dioxus::desktop::WindowCloseBehaviour::LastWindowExitsApp
    } else {
        tracing::debug!("In-process mode: using CloseWindow");
        dioxus::desktop::WindowCloseBehaviour::CloseWindow
    };

    let initial_state = AppState::with_filtering_state(db, filtering_state);

    // Store in global (will be consumed by App component)
    // Using Mutex so it can be replaced on subsequent launches
    if let Ok(mut guard) = INITIAL_STATE.lock() {
        *guard = Some(initial_state);
    }

    // Load window icon
    let icon = load_icon();

    tracing::debug!("run_dashboard_with_filtering: launching dioxus");

    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_window(
                    dioxus::desktop::WindowBuilder::new()
                        .with_title("Aegis Dashboard")
                        .with_inner_size(dioxus::desktop::LogicalSize::new(1000.0, 700.0))
                        .with_min_inner_size(dioxus::desktop::LogicalSize::new(800.0, 600.0))
                        .with_window_icon(icon),
                )
                .with_disable_context_menu(true)
                .with_close_behaviour(close_behaviour),
        )
        .launch(App);

    tracing::debug!("run_dashboard_with_filtering: dioxus launch returned");

    Ok(())
}

/// Loads the application icon.
fn load_icon() -> Option<dioxus::desktop::tao::window::Icon> {
    let icon_data = include_bytes!("../../aegis-app/assets/icons/icon-256.png");
    let image = image::load_from_memory(icon_data).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    dioxus::desktop::tao::window::Icon::from_rgba(image.into_raw(), width, height).ok()
}

/// Main application component.
#[allow(non_snake_case)]
fn App() -> Element {
    // Get initial state from global (take it, so it's fresh each launch)
    let initial_state = INITIAL_STATE
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
        .unwrap_or_else(|| {
            // Fallback for tests or if somehow not set
            let db = aegis_storage::Database::in_memory().expect("Failed to create in-memory database");
            AppState::new(db)
        });

    // Initialize global state
    use_context_provider(|| Signal::new(initial_state));

    // Get the current view
    let state = use_context::<Signal<AppState>>();
    let current_view = state.read().view;

    rsx! {
        style { {STYLES} }

        match current_view {
            View::Setup => rsx! { views::setup::SetupView {} },
            View::Login => rsx! { views::login::LoginView {} },
            _ => rsx! { AuthenticatedLayout {} },
        }
    }
}

/// Layout for authenticated views with sidebar.
#[component]
fn AuthenticatedLayout() -> Element {
    let state = use_context::<Signal<AppState>>();

    // Check session and get current view (read-only during render)
    let is_authenticated = state.read().is_authenticated();
    let current_view = state.read().view;

    // If not authenticated, redirect to login view
    // Note: The actual view change happens via navigation, not direct state write during render
    if !is_authenticated {
        return rsx! { views::login::LoginView {} };
    }

    rsx! {
        div { class: "app-container",
            // Sidebar
            components::sidebar::Sidebar {}

            // Main content
            main { class: "main-content",
                match current_view {
                    View::Dashboard => rsx! { views::dashboard::DashboardView {} },
                    View::Profiles => rsx! { views::profiles::ProfilesView {} },
                    View::Rules => rsx! { views::rules::RulesView {} },
                    View::Logs => rsx! { views::logs::LogsView {} },
                    View::Flagged => rsx! { views::flagged::FlaggedView {} },
                    View::SystemLogs => rsx! { views::system_logs::SystemLogsView {} },
                    View::Settings => rsx! { views::settings::SettingsView {} },
                    _ => rsx! { views::dashboard::DashboardView {} },
                }
            }
        }

        // Message toasts
        MessageToasts {}
    }
}

/// Toast messages for errors and success.
#[component]
fn MessageToasts() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let error_msg = state.read().error_message.clone();
    let success_msg = state.read().success_message.clone();

    rsx! {
        if let Some(ref error) = error_msg {
            div {
                class: "modal-overlay",
                style: "background: transparent; pointer-events: none;",
                div {
                    class: "auth-error",
                    style: "position: fixed; bottom: 20px; left: 50%; transform: translateX(-50%); pointer-events: auto; display: flex; align-items: center; gap: 8px;",
                    span { "{error}" }
                    button {
                        class: "btn btn-sm btn-secondary",
                        onclick: move |_| state.write().error_message = None,
                        "X"
                    }
                }
            }
        }

        if let Some(ref success) = success_msg {
            div {
                class: "modal-overlay",
                style: "background: transparent; pointer-events: none;",
                div {
                    class: "tag tag-success",
                    style: "position: fixed; bottom: 20px; left: 50%; transform: translateX(-50%); pointer-events: auto; display: flex; align-items: center; gap: 8px; padding: 8px 16px;",
                    span { "{success}" }
                    button {
                        class: "btn btn-sm btn-secondary",
                        onclick: move |_| state.write().success_message = None,
                        "X"
                    }
                }
            }
        }
    }
}

/// Placeholder for backwards compatibility with previous API.
pub mod settings {
    /// Placeholder type for settings UI functionality.
    pub struct SettingsUi;

    impl SettingsUi {
        /// Creates a new settings UI instance.
        pub fn new() -> Self {
            Self
        }
    }

    impl Default for SettingsUi {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_ui_can_be_created() {
        let _ui = settings::SettingsUi::new();
    }

    #[test]
    fn test_app_state_creation() {
        let db = aegis_storage::Database::in_memory().unwrap();
        let state = AppState::new(db);
        // First setup starts with Setup view
        assert_eq!(state.view, View::Setup);
    }

    #[test]
    fn test_protection_status() {
        assert_eq!(ProtectionStatus::Active.as_str(), "Active");
        assert_eq!(ProtectionStatus::Paused.as_str(), "Paused");
        assert_eq!(ProtectionStatus::Disabled.as_str(), "Disabled");
    }

    #[test]
    fn test_interception_mode() {
        assert_eq!(InterceptionMode::Proxy.as_str(), "Proxy");
    }
}

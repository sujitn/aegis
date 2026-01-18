//! Sidebar navigation component.

use dioxus::prelude::*;

use crate::state::{AppState, View};
use crate::components::icons::ShieldIcon;

/// Sidebar navigation component.
#[component]
pub fn Sidebar() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let current_view = state.read().view;
    let protection_status = state.read().protection_status();
    let flagged_count = state.read().unacknowledged_flagged_count();
    let status_class = protection_status.css_class();
    let status_str = protection_status.as_str();
    let version = env!("CARGO_PKG_VERSION");

    rsx! {
        aside { class: "sidebar",
            // Header with logo
            div { class: "sidebar-header",
                div { class: "sidebar-logo",
                    ShieldIcon { class: Some("sidebar-logo-icon".to_string()) }
                    span { class: "sidebar-logo-text", "Aegis" }
                }

                // Status pill
                div { class: "sidebar-status",
                    div { class: "status-pill",
                        span { class: "status-dot {status_class}" }
                        span { class: "status-text {status_class}", "{status_str}" }
                    }
                }
            }

            // Navigation
            nav { class: "sidebar-nav",
                NavItem {
                    label: "Dashboard",
                    icon: "dashboard",
                    active: current_view == View::Dashboard,
                    onclick: move |_| {
                        state.write().view = View::Dashboard;
                        let _ = state.write().refresh_data();
                    }
                }
                NavItem {
                    label: "Profiles",
                    icon: "users",
                    active: current_view == View::Profiles || current_view == View::Rules,
                    onclick: move |_| {
                        state.write().view = View::Profiles;
                        let _ = state.write().refresh_data();
                    }
                }
                NavItem {
                    label: "Activity",
                    icon: "list",
                    active: current_view == View::Logs,
                    onclick: move |_| {
                        state.write().view = View::Logs;
                        let _ = state.write().refresh_data();
                    }
                }
                NavItemWithBadge {
                    label: "Flagged",
                    icon: "flag",
                    active: current_view == View::Flagged,
                    badge_count: flagged_count,
                    onclick: move |_| {
                        state.write().view = View::Flagged;
                        let _ = state.write().refresh_data();
                    }
                }
                NavItem {
                    label: "System Logs",
                    icon: "terminal",
                    active: current_view == View::SystemLogs,
                    onclick: move |_| {
                        state.write().view = View::SystemLogs;
                    }
                }
                NavItem {
                    label: "Settings",
                    icon: "settings",
                    active: current_view == View::Settings,
                    onclick: move |_| {
                        state.write().view = View::Settings;
                    }
                }
            }

            // Footer
            div { class: "sidebar-footer",
                button {
                    class: "sidebar-footer-btn",
                    onclick: move |_| state.write().lock(),
                    "Lock Dashboard"
                }
                p { class: "sidebar-version", "v{version}" }
            }
        }
    }
}

/// Navigation item component.
#[component]
fn NavItem(
    label: &'static str,
    icon: &'static str,
    active: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let class = if active { "nav-item active" } else { "nav-item" };

    rsx! {
        div {
            class: "{class}",
            onclick: move |evt| onclick.call(evt),
            NavIcon { name: icon }
            span { "{label}" }
        }
    }
}

/// Navigation item with badge component.
#[component]
fn NavItemWithBadge(
    label: &'static str,
    icon: &'static str,
    active: bool,
    badge_count: i64,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let class = if active { "nav-item active" } else { "nav-item" };
    let badge_text = if badge_count > 99 {
        "99+".to_string()
    } else {
        badge_count.to_string()
    };

    rsx! {
        div {
            class: "{class}",
            onclick: move |evt| onclick.call(evt),
            NavIcon { name: icon }
            span { "{label}" }
            if badge_count > 0 {
                span { class: "nav-badge", "{badge_text}" }
            }
        }
    }
}

/// Simple navigation icon (text-based for now).
#[component]
fn NavIcon(name: &'static str) -> Element {
    let icon = match name {
        "dashboard" => "📊",
        "users" => "👥",
        "list" => "📋",
        "flag" => "🚩",
        "terminal" => "💻",
        "settings" => "⚙️",
        _ => "•",
    };

    rsx! {
        span { class: "nav-item-icon", "{icon}" }
    }
}

//! Dashboard home view.

use dioxus::prelude::*;

use crate::state::{AppState, View};
use crate::components::cards::{HeroCard, StatCard};

/// Dashboard view component.
#[component]
pub fn DashboardView() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let today_stats = state.read().today_stats.clone();
    let recent_events = state.read().recent_events.clone();
    let active_profile = state.read().get_active_profile().cloned();

    rsx! {
        div {
            // Profile indicator
            if let Some(ref profile) = active_profile {
                div { class: "profile-indicator",
                    span { class: "profile-indicator-label", "Current Profile:" }
                    span { class: "profile-badge",
                        span {
                            class: if profile.enabled { "profile-badge-dot active" } else { "profile-badge-dot inactive" }
                        }
                        "{profile.name}"
                    }
                    if !profile.enabled {
                        span { class: "tag tag-warning", style: "margin-left: 8px;", "Disabled" }
                    }
                }
            } else {
                div { class: "profile-indicator",
                    span { class: "profile-indicator-label", "No Active Profile" }
                    button {
                        class: "btn btn-secondary btn-sm",
                        style: "margin-left: 8px;",
                        onclick: move |_| state.write().view = View::Profiles,
                        "Create Profile"
                    }
                }
            }

            // Hero status card
            HeroCard {}

            // Stat cards row
            div { class: "stat-cards-grid",
                StatCard {
                    label: "Total Checked",
                    value: today_stats.as_ref().map(|s| s.total_prompts).unwrap_or(0),
                    color: "teal",
                    icon: "check-circle"
                }
                StatCard {
                    label: "Blocked",
                    value: today_stats.as_ref().map(|s| s.blocked_count).unwrap_or(0),
                    color: "coral",
                    icon: "x-circle"
                }
                StatCard {
                    label: "Warnings",
                    value: today_stats.as_ref().map(|s| s.flagged_count).unwrap_or(0),
                    color: "orange",
                    icon: "alert-triangle"
                }
                StatCard {
                    label: "Allowed",
                    value: today_stats.as_ref().map(|s| s.allowed_count).unwrap_or(0),
                    color: "green",
                    icon: "check"
                }
            }

            // Recent activity
            div { class: "activity-list",
                div { class: "activity-list-header",
                    h2 { class: "activity-list-title", "Recent Activity" }
                    button {
                        class: "btn btn-secondary btn-sm",
                        onclick: move |_| state.write().view = View::Logs,
                        "View All"
                    }
                }

                div { class: "card",
                    if recent_events.is_empty() {
                        div { class: "empty-state",
                            p { class: "empty-state-text", "No recent activity" }
                            p { class: "empty-state-subtext", "Events will appear here when prompts are checked" }
                        }
                    } else {
                        for event in recent_events.iter().take(10) {
                            ActivityItem {
                                preview: event.preview.clone(),
                                action: event.action,
                                source: event.source.clone(),
                                timestamp: event.created_at.format("%H:%M").to_string()
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Activity item component.
#[component]
fn ActivityItem(
    preview: String,
    action: aegis_storage::Action,
    source: Option<String>,
    timestamp: String,
) -> Element {
    let (icon, class) = match action {
        aegis_storage::Action::Allowed => ("✓", "allowed"),
        aegis_storage::Action::Blocked => ("✗", "blocked"),
        aegis_storage::Action::Flagged => ("!", "flagged"),
    };

    let preview_text = if preview.len() > 60 {
        format!("{}...", &preview[..60])
    } else {
        preview
    };

    rsx! {
        div { class: "activity-item",
            span { class: "activity-icon {class}", "{icon}" }
            span { class: "activity-text", "{preview_text}" }
            div { class: "activity-meta",
                if let Some(src) = source {
                    span { "{src}" }
                }
                span { "{timestamp}" }
            }
        }
    }
}

//! Card components for the dashboard.

use dioxus::prelude::*;

use crate::state::{AppState, ProtectionStatus};
use crate::components::icons::ShieldIcon;

/// Hero status card component.
#[component]
pub fn HeroCard() -> Element {
    let state = use_context::<Signal<AppState>>();
    let protection_status = state.read().protection_status;
    let today_stats = state.read().today_stats.clone();

    let (title, subtitle) = match protection_status {
        ProtectionStatus::Active => ("Your Family is Protected", "All systems active"),
        ProtectionStatus::Paused => ("Protection Paused", "Temporarily disabled"),
        ProtectionStatus::Disabled => ("Protection Disabled", "Your family is not protected"),
    };

    rsx! {
        div { class: "card-hero",
            div { class: "card-hero-content",
                // Shield icon
                div { class: "card-hero-icon {protection_status.css_class()}",
                    ShieldIcon { class: None }
                }

                div { class: "card-hero-info",
                    h1 { class: "card-hero-title {protection_status.css_class()}", "{title}" }
                    p { class: "card-hero-subtitle", "{subtitle}" }

                    // Mini stats row
                    if let Some(ref stats) = today_stats {
                        div { class: "card-hero-stats",
                            MiniStat { label: "Checked", value: stats.total_prompts, color: "white" }
                            MiniStat { label: "Blocked", value: stats.blocked_count, color: "coral" }
                            MiniStat { label: "Flagged", value: stats.flagged_count, color: "orange" }
                            MiniStat { label: "Allowed", value: stats.allowed_count, color: "green" }
                        }
                    }
                }
            }
        }
    }
}

/// Mini stat display for hero card.
#[component]
fn MiniStat(label: &'static str, value: i64, color: &'static str) -> Element {
    let color_class = match color {
        "coral" => "color: var(--aegis-coral-500);",
        "orange" => "color: var(--aegis-orange-400);",
        "green" => "color: var(--aegis-success);",
        _ => "color: white;",
    };

    rsx! {
        div { class: "mini-stat",
            span { class: "mini-stat-value", style: "{color_class}", "{value}" }
            span { class: "mini-stat-label", "{label}" }
        }
    }
}

/// Stat card component.
#[component]
pub fn StatCard(
    label: &'static str,
    value: i64,
    color: &'static str,
    icon: &'static str,
) -> Element {
    let color_class = match color {
        "teal" => "teal",
        "coral" => "coral",
        "orange" => "orange",
        "green" => "green",
        _ => "teal",
    };

    let icon_text = match icon {
        "check-circle" => "✓",
        "x-circle" => "✗",
        "alert-triangle" => "⚠",
        "check" => "✓",
        _ => "•",
    };

    rsx! {
        div { class: "stat-card",
            div { class: "stat-card-header",
                span { class: "stat-card-icon", "{icon_text}" }
                span { class: "stat-card-label", "{label}" }
            }
            div { class: "stat-card-value {color_class}", "{value}" }
        }
    }
}

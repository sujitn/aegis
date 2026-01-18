//! Card components for the dashboard.

use dioxus::prelude::*;

use aegis_core::protection::PauseDuration;

use crate::components::icons::ShieldIcon;
use crate::state::{AppState, ProtectionStatus};

/// Hero status card component with protection controls.
#[component]
pub fn HeroCard() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let protection_status = state.read().protection_status();
    let today_stats = state.read().today_stats.clone();
    let pause_remaining = state.read().pause_remaining_str();

    // State for showing pause dropdown
    let mut show_pause_menu = use_signal(|| false);

    let (title, subtitle) = match protection_status {
        ProtectionStatus::Active => ("Your Family is Protected", "All systems active"),
        ProtectionStatus::Paused => {
            if let Some(ref remaining) = pause_remaining {
                ("Protection Paused", remaining.as_str())
            } else {
                ("Protection Paused", "Paused indefinitely")
            }
        }
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

                // Protection controls
                div { class: "card-hero-actions",
                    match protection_status {
                        ProtectionStatus::Active => rsx! {
                            // Pause button with dropdown
                            div { class: "dropdown-container",
                                button {
                                    class: "btn btn-secondary",
                                    onclick: move |_| show_pause_menu.set(!show_pause_menu()),
                                    "Pause ▾"
                                }

                                if show_pause_menu() {
                                    PauseMenu {
                                        on_select: move |duration| {
                                            let result = state.write().pause_protection(duration);
                                            if let Err(e) = result {
                                                state.write().set_error(e.to_string());
                                            }
                                            show_pause_menu.set(false);
                                        },
                                        on_close: move |_| show_pause_menu.set(false),
                                        on_disable: move |_| {
                                            let result = state.write().disable_protection();
                                            if let Err(e) = result {
                                                state.write().set_error(e.to_string());
                                            }
                                            show_pause_menu.set(false);
                                        }
                                    }
                                }
                            }
                        },
                        ProtectionStatus::Paused => rsx! {
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| {
                                    state.write().resume_protection();
                                },
                                "Resume Now"
                            }
                        },
                        ProtectionStatus::Disabled => rsx! {
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| {
                                    state.write().resume_protection();
                                },
                                "Enable Protection"
                            }
                        },
                    }
                }
            }
        }
    }
}

/// Pause duration menu dropdown.
#[component]
fn PauseMenu(
    on_select: EventHandler<PauseDuration>,
    on_close: EventHandler<MouseEvent>,
    on_disable: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "dropdown-menu",
            // Click outside to close
            div {
                class: "dropdown-overlay",
                onclick: move |evt| on_close.call(evt)
            }

            div { class: "dropdown-content",
                p { class: "dropdown-title", "Pause Protection" }

                button {
                    class: "dropdown-item",
                    onclick: move |_| on_select.call(PauseDuration::FIVE_MINUTES),
                    "5 minutes"
                }
                button {
                    class: "dropdown-item",
                    onclick: move |_| on_select.call(PauseDuration::FIFTEEN_MINUTES),
                    "15 minutes"
                }
                button {
                    class: "dropdown-item",
                    onclick: move |_| on_select.call(PauseDuration::THIRTY_MINUTES),
                    "30 minutes"
                }
                button {
                    class: "dropdown-item",
                    onclick: move |_| on_select.call(PauseDuration::ONE_HOUR),
                    "1 hour"
                }
                button {
                    class: "dropdown-item",
                    onclick: move |_| on_select.call(PauseDuration::Indefinite),
                    "Until I resume"
                }

                hr { class: "dropdown-divider" }

                button {
                    class: "dropdown-item dropdown-item-danger",
                    onclick: move |evt| on_disable.call(evt),
                    "Disable completely"
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

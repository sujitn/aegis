//! Rules configuration view (simplified for Dioxus migration).

use dioxus::prelude::*;

use crate::state::{AppState, RulesTab, View};

/// Rules view component.
#[component]
pub fn RulesView() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let selected_profile_id = state.read().selected_profile_id;
    let rules_tab = state.read().rules_tab;
    let profiles = state.read().profiles.clone();

    // Get profile name
    let profile_name = selected_profile_id
        .and_then(|id| profiles.iter().find(|p| p.id == id))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    rsx! {
        div {
            // Header
            div { class: "flex items-center gap-md mb-lg",
                button {
                    class: "btn btn-secondary btn-sm",
                    onclick: move |_| state.write().view = View::Profiles,
                    "< Back"
                }
                h1 { class: "text-lg font-bold", "Rules: {profile_name}" }
            }

            // Tabs
            div { class: "tabs",
                div {
                    class: if rules_tab == RulesTab::Time { "tab active" } else { "tab" },
                    onclick: move |_| state.write().rules_tab = RulesTab::Time,
                    "Time Rules"
                }
                div {
                    class: if rules_tab == RulesTab::Content { "tab active" } else { "tab" },
                    onclick: move |_| state.write().rules_tab = RulesTab::Content,
                    "Content Rules"
                }
                div {
                    class: if rules_tab == RulesTab::Community { "tab active" } else { "tab" },
                    onclick: move |_| state.write().rules_tab = RulesTab::Community,
                    "Community Rules"
                }
            }

            // Tab content
            div { class: "card mt-md",
                match rules_tab {
                    RulesTab::Time => rsx! { TimeRulesTab {} },
                    RulesTab::Content => rsx! { ContentRulesTab {} },
                    RulesTab::Community => rsx! { CommunityRulesTab {} },
                }
            }
        }
    }
}

/// Time rules tab.
#[component]
fn TimeRulesTab() -> Element {
    rsx! {
        div {
            h3 { class: "font-bold mb-md", "Time-based Access Rules" }
            p { class: "text-muted mb-md", "Configure when AI access is allowed or blocked." }

            div { class: "empty-state",
                p { "Time rules configuration coming soon." }
                p { class: "text-sm text-muted", "Presets: School Night (9pm-7am), Weekend (11pm-8am)" }
            }
        }
    }
}

/// Content rules tab.
#[component]
fn ContentRulesTab() -> Element {
    rsx! {
        div {
            h3 { class: "font-bold mb-md", "Content Category Rules" }
            p { class: "text-muted mb-md", "Configure how each content category is handled." }

            // Category list
            ContentCategory { name: "Violence", description: "Violent content and threats", color: "var(--aegis-error)" }
            ContentCategory { name: "Self-Harm", description: "Self-harm and suicide content", color: "var(--aegis-error)" }
            ContentCategory { name: "Adult", description: "Sexual and adult material", color: "var(--aegis-warning)" }
            ContentCategory { name: "Jailbreak", description: "AI manipulation attempts", color: "var(--aegis-warning)" }
            ContentCategory { name: "Hate Speech", description: "Discriminatory content", color: "var(--aegis-error)" }
            ContentCategory { name: "Illegal", description: "Illegal activities", color: "var(--aegis-error)" }
            ContentCategory { name: "Profanity", description: "Offensive language", color: "var(--aegis-slate-400)" }
        }
    }
}

/// Content category row.
#[component]
fn ContentCategory(name: &'static str, description: &'static str, color: &'static str) -> Element {
    rsx! {
        div { class: "rule-card",
            span { class: "rule-category-dot", style: "background-color: {color};" }
            div { class: "rule-info",
                p { class: "rule-name", "{name}" }
                p { class: "rule-description", "{description}" }
            }
            div { class: "rule-controls",
                select { class: "select",
                    option { "Block" }
                    option { "Warn" }
                    option { "Allow" }
                }
            }
        }
    }
}

/// Community rules tab.
#[component]
fn CommunityRulesTab() -> Element {
    rsx! {
        div {
            h3 { class: "font-bold mb-md", "Community Rules" }
            p { class: "text-muted mb-md", "Rules from the Aegis community database." }

            div { class: "card mb-md",
                p { class: "font-bold", "Rule Priority (highest to lowest):" }
                p { "1. Parent (your customizations)" }
                p { "2. Curated (Aegis-maintained)" }
                p { "3. Community (open-source databases)" }
            }

            // Whitelist/Blacklist sections
            div { class: "mb-md",
                h4 { class: "font-bold mb-sm", "Whitelist (Never Block)" }
                p { class: "text-sm text-muted", "Terms in this list will never be blocked." }
                div { class: "flex gap-sm mt-sm",
                    input { class: "input", style: "flex: 1;", placeholder: "Add term..." }
                    button { class: "btn btn-primary btn-sm", "Add" }
                }
            }

            div { class: "mb-md",
                h4 { class: "font-bold mb-sm", "Blacklist (Always Block)" }
                p { class: "text-sm text-muted", "Add custom terms to always block." }
                div { class: "flex gap-sm mt-sm",
                    input { class: "input", style: "flex: 1;", placeholder: "Add term..." }
                    button { class: "btn btn-primary btn-sm", "Add" }
                }
            }
        }
    }
}

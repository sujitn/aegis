//! Profiles management view.

use dioxus::prelude::*;

use aegis_storage::ProfileSentimentConfig;

use crate::state::{AppState, View};

/// Calls the API to reload rules into the proxy.
fn reload_rules_from_api(profile_id: i64) {
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        let url = "http://127.0.0.1:48765/api/rules/reload";

        match client
            .post(url)
            .json(&serde_json::json!({ "profile_id": profile_id }))
            .send()
        {
            Ok(response) => {
                if response.status().is_success() {
                    tracing::info!("Rules reloaded in proxy for profile {}", profile_id);
                } else {
                    tracing::warn!("Failed to reload rules: HTTP {}", response.status());
                }
            }
            Err(e) => {
                tracing::warn!("Failed to call reload API: {}", e);
            }
        }
    });
}

/// Profiles view component.
#[component]
pub fn ProfilesView() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let profiles = state.read().profiles.clone();
    let mut show_editor = use_signal(|| false);
    let mut editor_profile_id = use_signal(|| None::<i64>);
    let mut editor_name = use_signal(String::new);
    let mut editor_os_username = use_signal(String::new);
    let mut editor_enabled = use_signal(|| true);
    let mut editor_sentiment_config = use_signal(ProfileSentimentConfig::default);
    let mut confirm_delete = use_signal(|| None::<i64>);

    rsx! {
        div {
            // Header
            div { class: "flex justify-between items-center mb-lg",
                h1 { class: "text-lg font-bold", "Profiles" }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| {
                        editor_profile_id.set(None);
                        editor_name.set(String::new());
                        editor_os_username.set(String::new());
                        editor_enabled.set(true);
                        editor_sentiment_config.set(ProfileSentimentConfig::default());
                        show_editor.set(true);
                    },
                    "+ New Profile"
                }
            }

            // Profile list
            if profiles.is_empty() {
                div { class: "card empty-state",
                    p { class: "empty-state-text", "No profiles yet" }
                    p { class: "empty-state-subtext", "Create a profile to start protecting a user" }
                    button {
                        class: "btn btn-primary mt-md",
                        onclick: move |_| {
                            editor_profile_id.set(None);
                            editor_name.set(String::new());
                            editor_os_username.set(String::new());
                            editor_enabled.set(true);
                            editor_sentiment_config.set(ProfileSentimentConfig::default());
                            show_editor.set(true);
                        },
                        "Create First Profile"
                    }
                }
            } else {
                for profile in profiles.iter() {
                    {
                        let profile_id = profile.id;
                        let profile_name = profile.name.clone();
                        let profile_enabled = profile.enabled;
                        let profile_os_username = profile.os_username.clone();
                        let is_confirming = confirm_delete() == Some(profile_id);

                        rsx! {
                            div { class: "profile-card",
                                span { class: if profile_enabled { "profile-status enabled" } else { "profile-status disabled" } }
                                div { class: "profile-info",
                                    p { class: "profile-name", "{profile_name}" }
                                    p { class: "profile-meta",
                                        if let Some(ref username) = profile_os_username {
                                            "OS User: {username}"
                                        } else {
                                            "Manual selection only"
                                        }
                                    }
                                }
                                div { class: "profile-actions",
                                    if is_confirming {
                                        span { class: "text-sm", style: "color: var(--aegis-error);", "Delete?" }
                                        button {
                                            class: "btn btn-danger btn-sm",
                                            onclick: move |_| {
                                                let delete_result = state.read().db.delete_profile(profile_id);
                                                if let Err(e) = delete_result {
                                                    state.write().set_error(e.to_string());
                                                } else {
                                                    let _ = state.write().refresh_data();
                                                    confirm_delete.set(None);
                                                }
                                            },
                                            "Yes"
                                        }
                                        button {
                                            class: "btn btn-secondary btn-sm",
                                            onclick: move |_| confirm_delete.set(None),
                                            "No"
                                        }
                                    } else {
                                        button {
                                            class: if profile_enabled { "btn btn-primary btn-sm" } else { "btn btn-secondary btn-sm" },
                                            onclick: move |_| {
                                                // Read current state and toggle
                                                let (_current_enabled, db_result) = {
                                                    let state_ref = state.read();
                                                    let current = state_ref.profiles
                                                        .iter()
                                                        .find(|p| p.id == profile_id)
                                                        .map(|p| p.enabled)
                                                        .unwrap_or(false);
                                                    let new_enabled = !current;
                                                    let result = state_ref.db.set_profile_enabled(profile_id, new_enabled);
                                                    (current, result)
                                                };

                                                // Handle result after releasing read lock
                                                match db_result {
                                                    Ok(()) => {
                                                        let _ = state.write().refresh_data();
                                                        reload_rules_from_api(profile_id);
                                                    }
                                                    Err(e) => {
                                                        state.write().set_error(e.to_string());
                                                    }
                                                }
                                            },
                                            if profile_enabled { "Enabled" } else { "Disabled" }
                                        }
                                        button {
                                            class: "btn btn-secondary btn-sm",
                                            onclick: move |_| {
                                                state.write().selected_profile_id = Some(profile_id);
                                                state.write().view = View::Rules;
                                            },
                                            "Rules"
                                        }
                                        button {
                                            class: "btn btn-secondary btn-sm",
                                            onclick: {
                                                let name_clone = profile_name.clone();
                                                let os_username_clone = profile_os_username.clone();
                                                // Get the sentiment config for this profile
                                                let sentiment_config = state.read().profiles
                                                    .iter()
                                                    .find(|p| p.id == profile_id)
                                                    .map(|p| p.sentiment_config.clone())
                                                    .unwrap_or_default();
                                                move |_| {
                                                    editor_profile_id.set(Some(profile_id));
                                                    editor_name.set(name_clone.clone());
                                                    editor_os_username.set(os_username_clone.clone().unwrap_or_default());
                                                    editor_enabled.set(profile_enabled);
                                                    editor_sentiment_config.set(sentiment_config.clone());
                                                    show_editor.set(true);
                                                }
                                            },
                                            "Edit"
                                        }
                                        button {
                                            class: "btn btn-secondary btn-sm",
                                            onclick: move |_| confirm_delete.set(Some(profile_id)),
                                            "Delete"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Editor modal
        if show_editor() {
            ProfileEditor {
                profile_id: editor_profile_id,
                name: editor_name,
                os_username: editor_os_username,
                enabled: editor_enabled,
                sentiment_config: editor_sentiment_config,
                state: state,
                on_close: move |_| show_editor.set(false),
                on_save: move |_| {
                    if save_profile(&mut state, &editor_profile_id, &editor_name, &editor_os_username, &editor_enabled, &editor_sentiment_config) {
                        show_editor.set(false);
                    }
                }
            }
        }
    }
}

/// Profile editor modal with sentiment analysis configuration.
#[component]
fn ProfileEditor(
    profile_id: Signal<Option<i64>>,
    name: Signal<String>,
    os_username: Signal<String>,
    enabled: Signal<bool>,
    sentiment_config: Signal<ProfileSentimentConfig>,
    state: Signal<AppState>,
    on_close: EventHandler<MouseEvent>,
    on_save: EventHandler<MouseEvent>,
) -> Element {
    let title = if profile_id().is_some() {
        "Edit Profile"
    } else {
        "New Profile"
    };

    // Sentiment config signals
    let mut sentiment_enabled = use_signal(|| sentiment_config().enabled);
    let mut sensitivity = use_signal(|| sentiment_config().sensitivity);
    let mut detect_distress = use_signal(|| sentiment_config().detect_distress);
    let mut detect_crisis = use_signal(|| sentiment_config().detect_crisis);
    let mut detect_bullying = use_signal(|| sentiment_config().detect_bullying);
    let mut detect_negative = use_signal(|| sentiment_config().detect_negative);

    // Sync sentiment signals to the parent signal when they change
    let mut sync_sentiment = move || {
        sentiment_config.set(ProfileSentimentConfig {
            enabled: sentiment_enabled(),
            sensitivity: sensitivity(),
            detect_distress: detect_distress(),
            detect_crisis: detect_crisis(),
            detect_bullying: detect_bullying(),
            detect_negative: detect_negative(),
        });
    };

    rsx! {
        div { class: "modal-overlay",
            div { class: "modal", style: "max-width: 550px;",
                div { class: "modal-header",
                    h3 { class: "modal-title", "{title}" }
                    button {
                        class: "modal-close",
                        onclick: move |evt| on_close.call(evt),
                        "×"
                    }
                }

                div { class: "modal-body",
                    // Basic profile settings
                    div { class: "mb-md",
                        label { class: "text-sm font-bold", "Name:" }
                        input {
                            class: "input",
                            value: "{name}",
                            oninput: move |evt| name.set(evt.value())
                        }
                    }

                    div { class: "mb-md",
                        label { class: "text-sm font-bold", "OS Username:" }
                        input {
                            class: "input",
                            placeholder: "Leave empty for manual selection",
                            value: "{os_username}",
                            oninput: move |evt| os_username.set(evt.value())
                        }
                        p { class: "text-sm text-muted mt-sm", "Automatically activates this profile for this OS user." }
                    }

                    div { class: "mb-lg",
                        label { class: "checkbox",
                            input {
                                r#type: "checkbox",
                                checked: "{enabled}",
                                onchange: move |evt| enabled.set(evt.checked())
                            }
                            "Enabled"
                        }
                    }

                    // Sentiment Analysis Section
                    div { class: "card", style: "background-color: var(--aegis-slate-900); padding: var(--spacing-md);",
                        h4 { class: "font-bold mb-md", "Sentiment Analysis" }

                        // Enable toggle
                        div { class: "mb-md",
                            label { class: "checkbox",
                                input {
                                    r#type: "checkbox",
                                    checked: "{sentiment_enabled}",
                                    onchange: move |evt| {
                                        sentiment_enabled.set(evt.checked());
                                        sync_sentiment();
                                    }
                                }
                                "Enable sentiment analysis"
                            }
                            p { class: "text-sm text-muted mt-sm", "Detect concerning emotional patterns in conversations." }
                        }

                        // Sensitivity selector
                        if sentiment_enabled() {
                            div { class: "mb-md",
                                label { class: "text-sm font-bold mb-sm", style: "display: block;", "Sensitivity:" }
                                div { class: "flex gap-sm",
                                    button {
                                        class: if sensitivity() <= 0.35 { "sensitivity-option selected" } else { "sensitivity-option" },
                                        onclick: move |_| {
                                            sensitivity.set(0.3);
                                            sync_sentiment();
                                        },
                                        "High"
                                    }
                                    button {
                                        class: if sensitivity() > 0.35 && sensitivity() <= 0.6 { "sensitivity-option selected" } else { "sensitivity-option" },
                                        onclick: move |_| {
                                            sensitivity.set(0.5);
                                            sync_sentiment();
                                        },
                                        "Medium"
                                    }
                                    button {
                                        class: if sensitivity() > 0.6 { "sensitivity-option selected" } else { "sensitivity-option" },
                                        onclick: move |_| {
                                            sensitivity.set(0.7);
                                            sync_sentiment();
                                        },
                                        "Low"
                                    }
                                }
                                p { class: "text-sm text-muted mt-sm",
                                    match sensitivity() {
                                        s if s <= 0.35 => "More sensitive - may flag more content",
                                        s if s > 0.6 => "Less sensitive - only flags clear concerns",
                                        _ => "Balanced detection (recommended)"
                                    }
                                }
                            }

                            // Detection toggles
                            div {
                                label { class: "text-sm font-bold mb-sm", style: "display: block;", "Detection Types:" }
                                div { class: "space-y-sm",
                                    label { class: "checkbox",
                                        input {
                                            r#type: "checkbox",
                                            checked: "{detect_distress}",
                                            onchange: move |evt| {
                                                detect_distress.set(evt.checked());
                                                sync_sentiment();
                                            }
                                        }
                                        "Detect emotional distress"
                                    }
                                    label { class: "checkbox",
                                        input {
                                            r#type: "checkbox",
                                            checked: "{detect_crisis}",
                                            onchange: move |evt| {
                                                detect_crisis.set(evt.checked());
                                                sync_sentiment();
                                            }
                                        }
                                        "Detect crisis indicators"
                                    }
                                    label { class: "checkbox",
                                        input {
                                            r#type: "checkbox",
                                            checked: "{detect_bullying}",
                                            onchange: move |evt| {
                                                detect_bullying.set(evt.checked());
                                                sync_sentiment();
                                            }
                                        }
                                        "Detect bullying discussion"
                                    }
                                    label { class: "checkbox",
                                        input {
                                            r#type: "checkbox",
                                            checked: "{detect_negative}",
                                            onchange: move |evt| {
                                                detect_negative.set(evt.checked());
                                                sync_sentiment();
                                            }
                                        }
                                        "Detect negative sentiment"
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "modal-footer",
                    button {
                        class: "btn btn-secondary",
                        onclick: move |evt| on_close.call(evt),
                        "Cancel"
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |evt| on_save.call(evt),
                        "Save"
                    }
                }
            }
        }
    }
}

/// Saves a profile.
fn save_profile(
    state: &mut Signal<AppState>,
    profile_id: &Signal<Option<i64>>,
    name: &Signal<String>,
    os_username: &Signal<String>,
    enabled: &Signal<bool>,
    sentiment_config: &Signal<ProfileSentimentConfig>,
) -> bool {
    let name_str = name().trim().to_string();
    if name_str.is_empty() {
        state.write().set_error("Profile name is required");
        return false;
    }

    let os_username_str = if os_username().trim().is_empty() {
        None
    } else {
        Some(os_username().trim().to_string())
    };

    // When editing, preserve existing rules; when creating, use empty rules
    let (time_rules, content_rules) = if let Some(id) = profile_id() {
        state
            .read()
            .profiles
            .iter()
            .find(|p| p.id == id)
            .map(|p| (p.time_rules.clone(), p.content_rules.clone()))
            .unwrap_or_else(|| {
                (
                    serde_json::json!({"rules": []}),
                    serde_json::json!({"rules": []}),
                )
            })
    } else {
        (
            serde_json::json!({"rules": []}),
            serde_json::json!({"rules": []}),
        )
    };

    let new_profile = aegis_storage::NewProfile {
        name: name_str,
        os_username: os_username_str,
        time_rules,
        content_rules,
        enabled: enabled(),
        sentiment_config: sentiment_config(),
    };

    let result = if let Some(id) = profile_id() {
        state.read().db.update_profile(id, new_profile)
    } else {
        state.read().db.create_profile(new_profile).map(|_| ())
    };

    match result {
        Ok(()) => {
            // Reload rules in proxy if editing existing profile
            if let Some(id) = profile_id() {
                reload_rules_from_api(id);
            }
            let _ = state.write().refresh_data();
            true
        }
        Err(e) => {
            state.write().set_error(e.to_string());
            false
        }
    }
}

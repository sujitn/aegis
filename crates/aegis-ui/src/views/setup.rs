//! First-run setup wizard view.

use std::env;

use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use dioxus::prelude::*;

use crate::state::{AppState, View};
use crate::components::icons::ShieldIcon;
use aegis_core::extension_install::get_extension_path;

/// App name for autostart.
const APP_NAME: &str = "Aegis";

/// Setup wizard steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SetupStep {
    #[default]
    Welcome,
    Password,
    ProtectionLevel,
    BrowserExtension,
    Profile,
    Complete,
}

#[allow(dead_code)]
impl SetupStep {
    fn number(&self) -> usize {
        match self {
            Self::Welcome => 1,
            Self::Password => 2,
            Self::ProtectionLevel => 3,
            Self::BrowserExtension => 4,
            Self::Profile => 5,
            Self::Complete => 6,
        }
    }

    fn total() -> usize {
        6
    }

    fn next(&self) -> Option<Self> {
        match self {
            Self::Welcome => Some(Self::Password),
            Self::Password => Some(Self::ProtectionLevel),
            Self::ProtectionLevel => Some(Self::BrowserExtension),
            Self::BrowserExtension => Some(Self::Profile),
            Self::Profile => Some(Self::Complete),
            Self::Complete => None,
        }
    }

    fn prev(&self) -> Option<Self> {
        match self {
            Self::Welcome => None,
            Self::Password => Some(Self::Welcome),
            Self::ProtectionLevel => Some(Self::Password),
            Self::BrowserExtension => Some(Self::ProtectionLevel),
            Self::Profile => Some(Self::BrowserExtension),
            Self::Complete => Some(Self::Profile),
        }
    }
}

/// Protection level presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProtectionLevel {
    #[default]
    Standard,
    Strict,
    Custom,
}

impl ProtectionLevel {
    fn name(&self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Strict => "Strict",
            Self::Custom => "Custom",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Standard => "Blocks harmful content like violence, adult content, and jailbreaks. Recommended for most families.",
            Self::Strict => "Blocks harmful content and flags questionable content for review. Better for younger children.",
            Self::Custom => "Start with no rules and configure everything yourself. For advanced users.",
        }
    }
}

/// Setup view component.
#[component]
pub fn SetupView() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let mut step = use_signal(|| SetupStep::Welcome);
    let password = use_signal(String::new);
    let confirm_password = use_signal(String::new);
    let protection_level = use_signal(|| ProtectionLevel::Standard);
    let profile_name = use_signal(String::new);
    let profile_os_username = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let enable_autostart = use_signal(|| true);

    let current_step = step();

    rsx! {
        div { class: "auth-container",
            // Progress indicator
            ProgressSteps { current: current_step.number(), total: SetupStep::total() }

            // Step content
            div { class: "auth-card", style: "max-width: 500px;",
                match current_step {
                    SetupStep::Welcome => rsx! {
                        WelcomeStep {
                            on_next: move |_| step.set(SetupStep::Password)
                        }
                    },
                    SetupStep::Password => rsx! {
                        PasswordStep {
                            password: password,
                            confirm_password: confirm_password,
                            error: error,
                            state: state,
                            on_next: move |_| {
                                if validate_and_save_password(&mut state, &password(), &confirm_password(), &mut error) {
                                    step.set(SetupStep::ProtectionLevel);
                                }
                            },
                            on_prev: move |_| step.set(SetupStep::Welcome)
                        }
                    },
                    SetupStep::ProtectionLevel => rsx! {
                        ProtectionLevelStep {
                            level: protection_level,
                            on_next: move |_| step.set(SetupStep::BrowserExtension),
                            on_prev: move |_| step.set(SetupStep::Password)
                        }
                    },
                    SetupStep::BrowserExtension => rsx! {
                        BrowserExtensionStep {
                            on_next: move |_| step.set(SetupStep::Profile),
                            on_prev: move |_| step.set(SetupStep::ProtectionLevel)
                        }
                    },
                    SetupStep::Profile => rsx! {
                        ProfileStep {
                            name: profile_name,
                            os_username: profile_os_username,
                            state: state,
                            protection_level: protection_level(),
                            error: error,
                            on_next: move |_| step.set(SetupStep::Complete),
                            on_prev: move |_| step.set(SetupStep::BrowserExtension)
                        }
                    },
                    SetupStep::Complete => rsx! {
                        CompleteStep {
                            enable_autostart: enable_autostart,
                            protection_level: protection_level(),
                            profile_name: profile_name(),
                            on_finish: move |_| {
                                finish_setup(&mut state, enable_autostart(), protection_level());
                            }
                        }
                    },
                }

                // Error message
                if let Some(err) = error() {
                    div { class: "auth-error mt-md", "{err}" }
                }
            }
        }
    }
}

/// Progress steps indicator.
#[component]
fn ProgressSteps(current: usize, total: usize) -> Element {
    rsx! {
        div { class: "progress-steps",
            for i in 1..=total {
                {
                    let is_current = i == current;
                    let is_done = i < current;
                    let step_class = if is_current { "progress-step current" }
                        else if is_done { "progress-step done" }
                        else { "progress-step" };
                    let line_class = if is_done { "progress-line done" } else { "progress-line" };

                    rsx! {
                        div { class: "{step_class}" }
                        if i < total {
                            div { class: "{line_class}" }
                        }
                    }
                }
            }
        }
        p { class: "progress-text", "Step {current} of {total}" }
    }
}

/// Welcome step.
#[component]
fn WelcomeStep(on_next: EventHandler<MouseEvent>) -> Element {
    rsx! {
        div { class: "text-center",
            ShieldIcon { class: "auth-logo-icon".to_string() }
            h2 { class: "auth-card-title mt-md", "Welcome to Aegis" }
            p { class: "text-muted mb-lg", "AI Safety for Families" }

            p { class: "mb-md", "Aegis helps protect your family from harmful AI interactions by:" }

            div { class: "text-left mb-lg", style: "padding-left: 20px;",
                p { "✓ Filtering dangerous prompts and jailbreak attempts" }
                p { "✓ Blocking inappropriate content categories" }
                p { "✓ Setting time-based usage rules" }
                p { "✓ Creating per-child protection profiles" }
                p { "✓ Logging activity for parental review" }
            }

            button {
                class: "btn btn-primary btn-lg",
                onclick: move |evt| on_next.call(evt),
                "Get Started"
            }
        }
    }
}

/// Password creation step.
#[component]
fn PasswordStep(
    password: Signal<String>,
    confirm_password: Signal<String>,
    error: Signal<Option<String>>,
    state: Signal<AppState>,
    on_next: EventHandler<MouseEvent>,
    on_prev: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "text-center",
            h2 { class: "auth-card-title", "Create Parent Password" }
            p { class: "text-muted text-sm mb-lg", "This password protects your Aegis settings from children." }

            div { class: "auth-form",
                input {
                    class: "input",
                    r#type: "password",
                    placeholder: "New Password (min 6 characters)",
                    value: "{password}",
                    oninput: move |evt| password.set(evt.value())
                }

                input {
                    class: "input",
                    r#type: "password",
                    placeholder: "Confirm Password",
                    value: "{confirm_password}",
                    oninput: move |evt| confirm_password.set(evt.value())
                }

                // Password strength indicator
                {
                    let len = password().len();
                    let (text, color) = if len == 0 { ("", "") }
                        else if len < 6 { ("Too short", "var(--aegis-error)") }
                        else if len < 10 { ("Acceptable", "var(--aegis-warning)") }
                        else { ("Strong", "var(--aegis-success)") };

                    if !text.is_empty() {
                        rsx! {
                            p { class: "text-sm", style: "color: {color};", "{text}" }
                        }
                    } else {
                        rsx! {}
                    }
                }

                div { class: "flex justify-between mt-md",
                    button {
                        class: "btn btn-secondary",
                        onclick: move |evt| on_prev.call(evt),
                        "Back"
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |evt| on_next.call(evt),
                        "Continue"
                    }
                }
            }
        }
    }
}

/// Protection level selection step.
#[component]
fn ProtectionLevelStep(
    level: Signal<ProtectionLevel>,
    on_next: EventHandler<MouseEvent>,
    on_prev: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "text-center",
            h2 { class: "auth-card-title", "Choose Protection Level" }
            p { class: "text-muted text-sm mb-lg", "Select how strictly Aegis should filter content." }

            div { class: "mb-lg",
                for opt in [ProtectionLevel::Standard, ProtectionLevel::Strict, ProtectionLevel::Custom] {
                    {
                        let is_selected = level() == opt;
                        let card_style = if is_selected {
                            "border: 2px solid var(--aegis-teal-500); background: var(--aegis-slate-700);"
                        } else {
                            "border: 2px solid transparent; background: var(--aegis-slate-800);"
                        };

                        rsx! {
                            div {
                                class: "card mb-sm",
                                style: "{card_style} cursor: pointer;",
                                onclick: move |_| level.set(opt),
                                div { class: "flex items-center gap-md",
                                    span {
                                        style: if is_selected { "color: var(--aegis-teal-400);" } else { "color: var(--aegis-slate-400);" },
                                        if is_selected { "●" } else { "○" }
                                    }
                                    div { class: "text-left",
                                        p { class: "font-bold", "{opt.name()}" }
                                        p { class: "text-sm text-muted", "{opt.description()}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "flex justify-between",
                button {
                    class: "btn btn-secondary",
                    onclick: move |evt| on_prev.call(evt),
                    "Back"
                }
                button {
                    class: "btn btn-primary",
                    onclick: move |evt| on_next.call(evt),
                    "Continue"
                }
            }
        }
    }
}

/// Browser extension installation step.
#[component]
fn BrowserExtensionStep(
    on_next: EventHandler<MouseEvent>,
    on_prev: EventHandler<MouseEvent>,
) -> Element {
    let ext_path = get_extension_path();

    rsx! {
        div { class: "text-center",
            h2 { class: "auth-card-title", "Install Browser Extension" }
            p { class: "text-muted text-sm mb-lg", "The browser extension monitors AI chatbots in Chrome, Edge, and other browsers." }

            if let Some(path) = ext_path {
                div { class: "text-left mb-lg",
                    p { class: "font-bold mb-sm", "Extension Location:" }
                    div { class: "card", style: "font-family: monospace; font-size: 11px; word-break: break-all;",
                        "{path.display()}"
                    }

                    p { class: "font-bold mt-md mb-sm", "Installation Steps:" }
                    ol { style: "padding-left: 20px;",
                        li { "Open chrome://extensions in your browser" }
                        li { "Enable 'Developer mode' (toggle in top-right)" }
                        li { "Click 'Load unpacked'" }
                        li { "Select the extension folder above" }
                        li { "The Aegis icon should appear in your toolbar" }
                    }

                    p { class: "text-sm text-muted mt-sm", "Supported: Chrome, Edge, Brave, Opera, Vivaldi" }
                }
            } else {
                p { class: "text-muted", "Extension folder not found. You can install it later from Settings." }
            }

            div { class: "flex justify-between",
                button {
                    class: "btn btn-secondary",
                    onclick: move |evt| on_prev.call(evt),
                    "Back"
                }
                button {
                    class: "btn btn-primary",
                    onclick: move |evt| on_next.call(evt),
                    "Continue"
                }
            }
        }
    }
}

/// Profile creation step.
#[component]
fn ProfileStep(
    name: Signal<String>,
    os_username: Signal<String>,
    state: Signal<AppState>,
    protection_level: ProtectionLevel,
    error: Signal<Option<String>>,
    on_next: EventHandler<MouseEvent>,
    on_prev: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "text-center",
            h2 { class: "auth-card-title", "Create First Profile" }
            p { class: "text-muted text-sm mb-lg", "Create a profile for a child you want to protect." }

            div { class: "auth-form",
                div { class: "text-left mb-sm",
                    label { class: "text-sm", "Name:" }
                    input {
                        class: "input",
                        placeholder: "e.g., Alex",
                        value: "{name}",
                        oninput: move |evt| name.set(evt.value())
                    }
                }

                div { class: "text-left mb-md",
                    label { class: "text-sm", "OS Username (optional):" }
                    input {
                        class: "input",
                        placeholder: "e.g., alex",
                        value: "{os_username}",
                        oninput: move |evt| os_username.set(evt.value())
                    }
                    p { class: "text-sm text-muted", "Auto-detect profile when this Windows/macOS user is logged in." }
                }

                div { class: "flex justify-between",
                    button {
                        class: "btn btn-secondary",
                        onclick: move |evt| on_prev.call(evt),
                        "Back"
                    }
                    div { class: "flex gap-sm",
                        button {
                            class: "btn btn-secondary",
                            onclick: move |evt| on_next.call(evt),
                            "Skip"
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: {
                                let state = state.clone();
                                let name = name.clone();
                                let os_username = os_username.clone();
                                let mut error = error.clone();
                                move |evt| {
                                    if create_profile(&state, &name(), &os_username(), protection_level, &mut error) {
                                        on_next.call(evt);
                                    }
                                }
                            },
                            "Create"
                        }
                    }
                }
            }
        }
    }
}

/// Complete step.
#[component]
fn CompleteStep(
    enable_autostart: Signal<bool>,
    protection_level: ProtectionLevel,
    profile_name: String,
    on_finish: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "text-center",
            h2 { class: "auth-card-title", style: "color: var(--aegis-success);", "Setup Complete!" }
            p { class: "mb-lg", style: "color: var(--aegis-success);", "Aegis is ready to protect your family." }

            div { class: "card text-left mb-lg",
                p { class: "font-bold mb-sm", "Your Configuration:" }
                p { "Protection Level: ", strong { "{protection_level.name()}" } }
                if !profile_name.is_empty() {
                    p { "Profile Created: ", strong { "{profile_name}" } }
                }
            }

            div { class: "text-left mb-lg",
                label { class: "checkbox",
                    input {
                        r#type: "checkbox",
                        checked: "{enable_autostart}",
                        onchange: move |evt| enable_autostart.set(evt.checked())
                    }
                    "Start Aegis when I log in"
                }
                p { class: "text-sm text-muted", "Aegis will start automatically in the background." }
            }

            button {
                class: "btn btn-primary btn-lg",
                onclick: move |evt| on_finish.call(evt),
                "Open Dashboard"
            }
        }
    }
}

/// Validates and saves password.
fn validate_and_save_password(
    state: &mut Signal<AppState>,
    password: &str,
    confirm: &str,
    error: &mut Signal<Option<String>>,
) -> bool {
    if password.len() < 6 {
        error.set(Some("Password must be at least 6 characters".to_string()));
        return false;
    }

    if password != confirm {
        error.set(Some("Passwords do not match".to_string()));
        return false;
    }

    // Hash the password first
    let hash_result = {
        let state_ref = state.read();
        state_ref.auth.hash_password(password)
    };

    match hash_result {
        Ok(hash) => {
            // Save the hash to the database
            let save_result = state.read().db.set_password_hash(&hash);
            if let Err(e) = save_result {
                error.set(Some(format!("Failed to save password: {}", e)));
                return false;
            }
            state.write().is_first_setup = false;
            error.set(None);
            true
        }
        Err(e) => {
            error.set(Some(format!("Failed to hash password: {}", e)));
            false
        }
    }
}

/// Creates a profile.
fn create_profile(
    state: &Signal<AppState>,
    name: &str,
    os_username: &str,
    level: ProtectionLevel,
    error: &mut Signal<Option<String>>,
) -> bool {
    let name = name.trim();
    if name.is_empty() {
        error.set(Some("Profile name is required".to_string()));
        return false;
    }

    let os_username = if os_username.trim().is_empty() {
        None
    } else {
        Some(os_username.trim().to_string())
    };

    let (time_rules, content_rules) = create_default_rules(level);

    let new_profile = aegis_storage::NewProfile {
        name: name.to_string(),
        os_username,
        time_rules,
        content_rules,
        enabled: true,
        sentiment_config: aegis_storage::ProfileSentimentConfig::default(),
    };

    match state.read().db.create_profile(new_profile) {
        Ok(_) => {
            error.set(None);
            true
        }
        Err(e) => {
            error.set(Some(format!("Failed to create profile: {}", e)));
            false
        }
    }
}

/// Creates default rules based on protection level.
fn create_default_rules(level: ProtectionLevel) -> (serde_json::Value, serde_json::Value) {
    match level {
        ProtectionLevel::Standard => {
            let time_rules = serde_json::json!({
                "rules": [{
                    "name": "School Night Bedtime",
                    "enabled": true,
                    "start_time": "21:00",
                    "end_time": "07:00",
                    "days": ["monday", "tuesday", "wednesday", "thursday", "sunday"]
                }]
            });

            let content_rules = serde_json::json!({
                "rules": [
                    {"category": "violence", "action": "block", "threshold": 0.7},
                    {"category": "self_harm", "action": "block", "threshold": 0.5},
                    {"category": "adult", "action": "block", "threshold": 0.7},
                    {"category": "jailbreak", "action": "block", "threshold": 0.6},
                    {"category": "hate", "action": "block", "threshold": 0.7},
                    {"category": "illegal", "action": "block", "threshold": 0.7}
                ]
            });

            (time_rules, content_rules)
        }
        ProtectionLevel::Strict => {
            let time_rules = serde_json::json!({
                "rules": [{
                    "name": "Early Bedtime",
                    "enabled": true,
                    "start_time": "20:00",
                    "end_time": "08:00",
                    "days": ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"]
                }]
            });

            let content_rules = serde_json::json!({
                "rules": [
                    {"category": "violence", "action": "block", "threshold": 0.5},
                    {"category": "self_harm", "action": "block", "threshold": 0.3},
                    {"category": "adult", "action": "block", "threshold": 0.5},
                    {"category": "jailbreak", "action": "block", "threshold": 0.4},
                    {"category": "hate", "action": "block", "threshold": 0.5},
                    {"category": "illegal", "action": "block", "threshold": 0.5}
                ]
            });

            (time_rules, content_rules)
        }
        ProtectionLevel::Custom => {
            (serde_json::json!({"rules": []}), serde_json::json!({"rules": []}))
        }
    }
}

/// Finishes the setup wizard.
fn finish_setup(state: &mut Signal<AppState>, enable_autostart: bool, level: ProtectionLevel) {
    let level_str = match level {
        ProtectionLevel::Standard => "standard",
        ProtectionLevel::Strict => "strict",
        ProtectionLevel::Custom => "custom",
    };

    // Save settings
    let _ = state.read().db.set_config("interception_mode", &serde_json::json!("proxy"));
    let _ = state.read().db.set_config("protection_level", &serde_json::json!(level_str));
    let _ = state.read().db.set_config("setup_complete", &serde_json::json!(true));

    // Enable autostart if requested
    if enable_autostart {
        if let Err(e) = enable_autostart_fn() {
            tracing::warn!("Failed to enable autostart: {}", e);
        }
    }

    // Create session and go to dashboard
    let token = state.read().auth.create_session();
    state.write().session = Some(token);
    state.write().view = View::Dashboard;
    let _ = state.write().refresh_data();
}

/// Creates an AutoLaunch instance.
fn create_auto_launch() -> Option<AutoLaunch> {
    let exe_path = env::current_exe().ok()?;
    let exe_str = exe_path.to_str()?;
    let args = &["--minimized"];

    #[cfg(target_os = "macos")]
    let launcher = AutoLaunchBuilder::new()
        .set_app_name(APP_NAME)
        .set_app_path(exe_str)
        .set_args(args)
        .set_use_launch_agent(true)
        .build()
        .ok()?;

    #[cfg(not(target_os = "macos"))]
    let launcher = AutoLaunchBuilder::new()
        .set_app_name(APP_NAME)
        .set_app_path(exe_str)
        .set_args(args)
        .build()
        .ok()?;

    Some(launcher)
}

/// Enables autostart.
fn enable_autostart_fn() -> Result<(), String> {
    let launcher = create_auto_launch().ok_or("Failed to create autostart")?;
    launcher.enable().map_err(|e| e.to_string())
}

//! First-run setup wizard view.

#![allow(clippy::clone_on_copy)]

use std::env;

use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use dioxus::prelude::*;

use crate::components::icons::ShieldIcon;
use crate::state::{AppState, View};
use aegis_core::extension_install::get_extension_path;
use aegis_core::model_downloader::{self, ModelDownloader, MlStatus};
use aegis_proxy::setup::{
    enable_system_proxy, install_ca_certificate, is_ca_installed, is_proxy_enabled,
};
use aegis_proxy::{CaManager, DEFAULT_PROXY_PORT};

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
    MitmSetup,
    ImageFiltering,
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
            Self::MitmSetup => 5,
            Self::ImageFiltering => 6,
            Self::Profile => 7,
            Self::Complete => 8,
        }
    }

    fn total() -> usize {
        8
    }

    fn next(&self) -> Option<Self> {
        match self {
            Self::Welcome => Some(Self::Password),
            Self::Password => Some(Self::ProtectionLevel),
            Self::ProtectionLevel => Some(Self::BrowserExtension),
            Self::BrowserExtension => Some(Self::MitmSetup),
            Self::MitmSetup => Some(Self::ImageFiltering),
            Self::ImageFiltering => Some(Self::Profile),
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
            Self::MitmSetup => Some(Self::BrowserExtension),
            Self::ImageFiltering => Some(Self::MitmSetup),
            Self::Profile => Some(Self::ImageFiltering),
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

/// Proxy configuration constants.
const PROXY_HOST: &str = "127.0.0.1";

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

    // MITM setup state
    let ca_installed = use_signal(|| false);
    let proxy_enabled = use_signal(|| false);
    let ca_installing = use_signal(|| false);
    let proxy_configuring = use_signal(|| false);

    // ML status state
    let ml_status = use_signal(|| model_downloader::get_ml_status());
    let ml_downloading = use_signal(|| false);
    let ml_progress_text = use_signal(|| String::new());

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
                            on_next: move |_| step.set(SetupStep::MitmSetup),
                            on_prev: move |_| step.set(SetupStep::ProtectionLevel)
                        }
                    },
                    SetupStep::MitmSetup => rsx! {
                        MitmSetupStep {
                            ca_installed: ca_installed,
                            proxy_enabled: proxy_enabled,
                            ca_installing: ca_installing,
                            proxy_configuring: proxy_configuring,
                            on_next: move |_| step.set(SetupStep::ImageFiltering),
                            on_prev: move |_| step.set(SetupStep::BrowserExtension)
                        }
                    },
                    SetupStep::ImageFiltering => rsx! {
                        ImageFilteringStep {
                            ml_status: ml_status,
                            ml_downloading: ml_downloading,
                            ml_progress_text: ml_progress_text,
                            on_next: move |_| step.set(SetupStep::Profile),
                            on_prev: move |_| step.set(SetupStep::MitmSetup)
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
                            on_prev: move |_| step.set(SetupStep::ImageFiltering)
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

/// MITM setup step (CA certificate and proxy).
#[component]
fn MitmSetupStep(
    mut ca_installed: Signal<bool>,
    mut proxy_enabled: Signal<bool>,
    mut ca_installing: Signal<bool>,
    mut proxy_configuring: Signal<bool>,
    on_next: EventHandler<MouseEvent>,
    on_prev: EventHandler<MouseEvent>,
) -> Element {
    // Get CA certificate path
    let ca_path = CaManager::with_default_dir()
        .ok()
        .map(|m| m.cert_path())
        .filter(|p| p.exists());

    // Check status on mount
    let ca_path_for_effect = ca_path.clone();
    use_effect(move || {
        let ca_path_clone = ca_path_for_effect.clone();
        spawn(async move {
            let ca_installed_val = ca_path_clone
                .as_ref()
                .map(|p| is_ca_installed(p))
                .unwrap_or(false);
            let proxy_enabled_val = is_proxy_enabled(PROXY_HOST, DEFAULT_PROXY_PORT);
            ca_installed.set(ca_installed_val);
            proxy_enabled.set(proxy_enabled_val);
        });
    });

    rsx! {
        div { class: "text-center",
            h2 { class: "auth-card-title", "System Proxy Setup" }
            p { class: "text-muted text-sm mb-lg", "For full protection, install the CA certificate and enable system proxy. This allows Aegis to filter all AI traffic, not just browser extensions." }

            div { class: "text-left mb-lg",
                // CA Certificate Section
                div { class: "card mb-md", style: "background-color: var(--aegis-slate-800);",
                    div { class: "flex items-center gap-sm mb-md",
                        span { style: "font-size: 20px;", "🔐" }
                        span { class: "font-bold", "CA Certificate" }
                        if ca_installed() {
                            span { class: "tag tag-success", "Installed" }
                        } else {
                            span { class: "tag tag-warning", "Required" }
                        }
                    }

                    if let Some(ref path) = ca_path {
                        if !ca_installed() {
                            button {
                                class: "btn btn-primary btn-sm",
                                disabled: ca_installing(),
                                onclick: {
                                    let path_clone = path.clone();
                                    move |_| {
                                        ca_installing.set(true);
                                        let result = install_ca_certificate(&path_clone);
                                        ca_installing.set(false);
                                        if result.success {
                                            ca_installed.set(true);
                                        }
                                    }
                                },
                                if ca_installing() { "Installing..." } else { "Install Certificate" }
                            }
                            p { class: "text-sm text-muted mt-sm", "This will prompt for administrator privileges." }
                        } else {
                            p { class: "text-sm text-success", "Certificate is installed and trusted." }
                        }
                    } else {
                        p { class: "text-sm text-muted", "Certificate not generated yet. Start the app once to generate it." }
                    }
                }

                // System Proxy Section
                div { class: "card", style: "background-color: var(--aegis-slate-800);",
                    div { class: "flex items-center gap-sm mb-md",
                        span { style: "font-size: 20px;", "🌐" }
                        span { class: "font-bold", "System Proxy" }
                        if proxy_enabled() {
                            span { class: "tag tag-success", "Enabled" }
                        } else {
                            span { class: "tag tag-secondary", "Optional" }
                        }
                    }

                    if ca_installed() {
                        if !proxy_enabled() {
                            button {
                                class: "btn btn-primary btn-sm",
                                disabled: proxy_configuring(),
                                onclick: move |_| {
                                    proxy_configuring.set(true);
                                    let result = enable_system_proxy(PROXY_HOST, DEFAULT_PROXY_PORT);
                                    proxy_configuring.set(false);
                                    if result.success {
                                        proxy_enabled.set(true);
                                    }
                                },
                                if proxy_configuring() { "Enabling..." } else { "Enable System Proxy" }
                            }
                            p { class: "text-sm text-muted mt-sm", "Routes all system traffic through Aegis for filtering." }
                        } else {
                            p { class: "text-sm text-success", "System proxy is enabled. All traffic is being filtered." }
                        }
                    } else {
                        p { class: "text-sm text-muted", "Install the CA certificate first to enable proxy." }
                    }
                }
            }

            p { class: "text-sm text-muted mb-md", "You can skip this step and configure it later in Settings." }

            div { class: "flex justify-between",
                button {
                    class: "btn btn-secondary",
                    onclick: move |evt| on_prev.call(evt),
                    "Back"
                }
                button {
                    class: "btn btn-primary",
                    onclick: move |evt| on_next.call(evt),
                    if ca_installed() && proxy_enabled() { "Continue" } else { "Skip" }
                }
            }
        }
    }
}

/// Image filtering (ML) setup step.
#[component]
fn ImageFilteringStep(
    mut ml_status: Signal<MlStatus>,
    mut ml_downloading: Signal<bool>,
    mut ml_progress_text: Signal<String>,
    on_next: EventHandler<MouseEvent>,
    on_prev: EventHandler<MouseEvent>,
) -> Element {
    // Refresh status on mount
    use_effect(move || {
        ml_status.set(model_downloader::get_ml_status());
    });

    let is_ready = matches!(ml_status(), MlStatus::Ready);

    rsx! {
        div { class: "text-center",
            h2 { class: "auth-card-title", "Image Content Filtering" }
            p { class: "text-muted text-sm mb-lg", "Aegis can filter inappropriate images using machine learning. This requires downloading additional components (~50MB)." }

            div { class: "text-left mb-lg",
                div { class: "card", style: "background-color: var(--aegis-slate-800);",
                    div { class: "flex items-center gap-sm mb-md",
                        span { style: "font-size: 20px;", "🖼️" }
                        span { class: "font-bold", "NSFW Image Detection" }
                        match ml_status() {
                            MlStatus::Ready => rsx! {
                                span { class: "tag tag-success", "Ready" }
                            },
                            MlStatus::Downloading { .. } => rsx! {
                                span { class: "tag tag-info", "Downloading" }
                            },
                            MlStatus::Failed { .. } => rsx! {
                                span { class: "tag tag-danger", "Failed" }
                            },
                            _ => rsx! {
                                span { class: "tag tag-warning", "Not Installed" }
                            }
                        }
                    }

                    p { class: "text-sm text-muted mb-md", "Detects and blocks inappropriate images in AI responses using an on-device ML model. No data is sent to external servers." }

                    // Download progress text
                    if !ml_progress_text().is_empty() {
                        div { class: "card mb-md", style: "background-color: var(--aegis-slate-900);",
                            p { class: "text-sm", "{ml_progress_text}" }
                        }
                    }

                    // Show downloading spinner
                    if ml_downloading() {
                        div { class: "flex items-center gap-sm mb-md",
                            div { class: "spinner" }
                            span { class: "text-sm text-muted", "Downloading..." }
                        }
                    }

                    if is_ready {
                        p { class: "text-sm text-success", "Image filtering is ready to use." }
                    } else if !ml_downloading() {
                        button {
                            class: "btn btn-primary btn-sm",
                            onclick: move |_| {
                                ml_downloading.set(true);
                                ml_progress_text.set("Downloading ONNX Runtime and NSFW model...".to_string());
                                ml_status.set(MlStatus::Downloading {
                                    step: "Downloading...".to_string(),
                                    progress: None,
                                });

                                spawn(async move {
                                    let Some(downloader) = ModelDownloader::new() else {
                                        ml_status.set(MlStatus::Failed {
                                            error: "Failed to initialize downloader".to_string(),
                                        });
                                        ml_downloading.set(false);
                                        ml_progress_text.set(String::new());
                                        return;
                                    };

                                    match downloader.ensure_all(None).await {
                                        Ok(()) => {
                                            ml_status.set(MlStatus::Ready);
                                            ml_progress_text.set("Download complete!".to_string());
                                            downloader.setup_environment();
                                        }
                                        Err(e) => {
                                            ml_status.set(MlStatus::Failed {
                                                error: e.to_string(),
                                            });
                                            ml_progress_text.set(format!("Download failed: {}", e));
                                        }
                                    }
                                    ml_downloading.set(false);
                                });
                            },
                            "Download ML Components"
                        }
                    }

                    if let MlStatus::Failed { error } = ml_status() {
                        p { class: "text-sm text-danger mt-sm", "{error}" }
                    }
                }
            }

            p { class: "text-sm text-muted mb-md", "You can skip this step and download later in Settings." }

            div { class: "flex justify-between",
                button {
                    class: "btn btn-secondary",
                    onclick: move |evt| on_prev.call(evt),
                    "Back"
                }
                button {
                    class: "btn btn-primary",
                    onclick: move |evt| on_next.call(evt),
                    if is_ready { "Continue" } else { "Skip" }
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
        image_filtering_config: aegis_storage::ProfileImageFilteringConfig::default(),
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
                    {"id": "violence_block", "name": "Block Violence", "category": "violence", "action": "block", "threshold": 0.7, "enabled": true},
                    {"id": "selfharm_block", "name": "Block Self-Harm", "category": "self_harm", "action": "block", "threshold": 0.5, "enabled": true},
                    {"id": "adult_block", "name": "Block Adult Content", "category": "adult", "action": "block", "threshold": 0.7, "enabled": true},
                    {"id": "jailbreak_block", "name": "Block Jailbreak", "category": "jailbreak", "action": "block", "threshold": 0.6, "enabled": true},
                    {"id": "hate_block", "name": "Block Hate Speech", "category": "hate", "action": "block", "threshold": 0.7, "enabled": true},
                    {"id": "illegal_block", "name": "Block Illegal Content", "category": "illegal", "action": "block", "threshold": 0.7, "enabled": true},
                    {"id": "profanity_block", "name": "Block Profanity", "category": "profanity", "action": "block", "threshold": 0.6, "enabled": true}
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
                    {"id": "violence_block", "name": "Block Violence", "category": "violence", "action": "block", "threshold": 0.5, "enabled": true},
                    {"id": "selfharm_block", "name": "Block Self-Harm", "category": "self_harm", "action": "block", "threshold": 0.3, "enabled": true},
                    {"id": "adult_block", "name": "Block Adult Content", "category": "adult", "action": "block", "threshold": 0.5, "enabled": true},
                    {"id": "jailbreak_block", "name": "Block Jailbreak", "category": "jailbreak", "action": "block", "threshold": 0.4, "enabled": true},
                    {"id": "hate_block", "name": "Block Hate Speech", "category": "hate", "action": "block", "threshold": 0.5, "enabled": true},
                    {"id": "illegal_block", "name": "Block Illegal Content", "category": "illegal", "action": "block", "threshold": 0.5, "enabled": true},
                    {"id": "profanity_block", "name": "Block Profanity", "category": "profanity", "action": "block", "threshold": 0.5, "enabled": true}
                ]
            });

            (time_rules, content_rules)
        }
        ProtectionLevel::Custom => (
            serde_json::json!({"rules": []}),
            serde_json::json!({"rules": []}),
        ),
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
    let _ = state
        .read()
        .db
        .set_config("interception_mode", &serde_json::json!("proxy"));
    let _ = state
        .read()
        .db
        .set_config("protection_level", &serde_json::json!(level_str));
    let _ = state
        .read()
        .db
        .set_config("setup_complete", &serde_json::json!(true));

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

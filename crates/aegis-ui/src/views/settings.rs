//! Settings view.

use std::env;

use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use dioxus::prelude::*;

use aegis_core::extension_install::get_extension_path;
use aegis_core::model_downloader::{self, ModelDownloader, MlStatus};
use aegis_proxy::setup::{
    disable_system_proxy, enable_system_proxy, install_ca_certificate, is_ca_installed,
    is_proxy_enabled, uninstall_ca_certificate,
};
use aegis_proxy::{CaManager, DEFAULT_PROXY_PORT};

use crate::state::AppState;

/// App name for autostart.
const APP_NAME: &str = "Aegis";

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

/// Checks if autostart is enabled.
fn is_autostart_enabled() -> bool {
    create_auto_launch()
        .map(|l| l.is_enabled().unwrap_or(false))
        .unwrap_or(false)
}

/// Interception mode for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InterceptionMode {
    Extension,
    Proxy,
}

/// Settings view component.
#[component]
pub fn SettingsView() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let mut loading = use_signal(|| true);
    let mut autostart = use_signal(|| false);
    let mut show_change_password = use_signal(|| false);
    let mut current_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut interception_mode = use_signal(|| InterceptionMode::Extension);
    let mut show_ca_instructions = use_signal(|| false);
    let mut show_proxy_instructions = use_signal(|| false);
    let mut ca_installing = use_signal(|| false);
    let mut proxy_configuring = use_signal(|| false);

    // ML status state
    let mut ml_status = use_signal(|| model_downloader::get_ml_status());
    let mut ml_downloading = use_signal(|| false);
    let mut ml_progress_text = use_signal(|| String::new());

    // Proxy configuration constants
    const PROXY_HOST: &str = "127.0.0.1";

    // Cache paths (computed once - these are fast)
    let ext_path = get_extension_path();
    let ext_path_display = ext_path.as_ref().map(|p| p.display().to_string());
    let version = env!("CARGO_PKG_VERSION");

    // Get CA certificate path (computed once - fast)
    let ca_path = CaManager::with_default_dir()
        .ok()
        .map(|m| m.cert_path())
        .filter(|p| p.exists());
    let ca_path_display = ca_path.as_ref().map(|p| p.display().to_string());

    // Deferred status checks - these are slow system calls
    let mut ca_installed_status = use_signal(|| false);
    let mut proxy_enabled_status = use_signal(|| false);

    // Clone ca_path for use in the effect closure
    let ca_path_for_effect = ca_path.clone();

    // Load slow system checks asynchronously on mount
    use_effect(move || {
        let ca_path_clone = ca_path_for_effect.clone();
        spawn(async move {
            // Run slow checks in background
            let autostart_enabled = is_autostart_enabled();
            let ca_installed = ca_path_clone
                .as_ref()
                .map(|p| is_ca_installed(p))
                .unwrap_or(false);
            let proxy_enabled = is_proxy_enabled(PROXY_HOST, DEFAULT_PROXY_PORT);

            // Update signals
            autostart.set(autostart_enabled);
            ca_installed_status.set(ca_installed);
            proxy_enabled_status.set(proxy_enabled);
            loading.set(false);
        });
    });

    let ca_installed = ca_installed_status();
    let proxy_enabled = proxy_enabled_status();

    // Show loading indicator while checking system status
    if loading() {
        return rsx! {
            div {
                h1 { class: "text-lg font-bold mb-lg", "Settings" }
                div { class: "card",
                    div { class: "flex items-center justify-center gap-md py-xl",
                        div { class: "spinner" }
                        span { class: "text-muted", "Loading settings..." }
                    }
                }
            }
        };
    }

    rsx! {
        div {
            h1 { class: "text-lg font-bold mb-lg", "Settings" }

            // General section
            div { class: "card mb-lg",
                h2 { class: "font-bold mb-md", "General" }

                div { class: "flex justify-between items-center mb-md",
                    div {
                        p { "Start on login" }
                        p { class: "text-sm text-muted", "Aegis will start automatically when you log in." }
                    }
                    label { class: "checkbox",
                        input {
                            r#type: "checkbox",
                            checked: "{autostart}",
                            onchange: move |evt| {
                                let enabled = evt.checked();
                                if let Some(launcher) = create_auto_launch() {
                                    let result = if enabled {
                                        launcher.enable()
                                    } else {
                                        launcher.disable()
                                    };
                                    match result {
                                        Ok(()) => {
                                            autostart.set(enabled);
                                            state.write().set_success(if enabled { "Autostart enabled" } else { "Autostart disabled" });
                                        }
                                        Err(e) => state.write().set_error(e.to_string()),
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Interception Mode section
            div { class: "card mb-lg",
                h2 { class: "font-bold mb-md", "Interception Mode" }
                p { class: "text-sm text-muted mb-md", "Choose how Aegis monitors AI interactions." }

                div { class: "flex gap-md mb-md",
                    // Extension Mode Card
                    div {
                        class: if interception_mode() == InterceptionMode::Extension { "mode-card selected" } else { "mode-card" },
                        style: "flex: 1;",
                        onclick: move |_| interception_mode.set(InterceptionMode::Extension),
                        div { class: "flex items-center gap-sm mb-sm",
                            span { style: "font-size: 20px;", "🧩" }
                            span { class: "font-bold", "Browser Extension" }
                        }
                        p { class: "text-sm text-muted", "Protects browser-based AI tools only. Easy setup, no certificate required." }
                    }

                    // Proxy Mode Card
                    div {
                        class: if interception_mode() == InterceptionMode::Proxy { "mode-card selected" } else { "mode-card" },
                        style: "flex: 1;",
                        onclick: move |_| interception_mode.set(InterceptionMode::Proxy),
                        div { class: "flex items-center gap-sm mb-sm",
                            span { style: "font-size: 20px;", "🔒" }
                            span { class: "font-bold", "System Proxy" }
                        }
                        p { class: "text-sm text-muted", "Protects all applications. Requires CA certificate installation." }
                    }
                }

                // CA Certificate Panel (shown when Proxy mode selected)
                if interception_mode() == InterceptionMode::Proxy {
                    div { class: "card", style: "background-color: var(--aegis-slate-900);",
                        div { class: "flex items-center gap-sm mb-md",
                            if ca_installed {
                                span { class: "tag tag-success", "CA Certificate Installed" }
                            } else {
                                span { class: "tag tag-warning", "CA Certificate Required" }
                            }
                        }

                        if let Some(ref path_str) = ca_path_display {
                            // Auto-install button
                            if !ca_installed {
                                div { class: "mb-md",
                                    button {
                                        class: "btn btn-primary",
                                        disabled: ca_installing(),
                                        onclick: {
                                            let ca_path_clone = ca_path.clone();
                                            move |_| {
                                                if let Some(ref path) = ca_path_clone {
                                                    ca_installing.set(true);
                                                    let result = install_ca_certificate(path);
                                                    ca_installing.set(false);
                                                    if result.success {
                                                        state.write().set_success(&result.message);
                                                        ca_installed_status.set(true);
                                                    } else {
                                                        state.write().set_error(&result.message);
                                                    }
                                                }
                                            }
                                        },
                                        if ca_installing() { "Installing..." } else { "Install Certificate Automatically" }
                                    }
                                    p { class: "text-sm text-muted mt-sm", "This will prompt for administrator privileges." }
                                }
                            } else {
                                div { class: "mb-md",
                                    p { class: "text-sm text-success mb-sm", "Certificate is installed and trusted by the system." }
                                    button {
                                        class: "btn btn-danger btn-sm",
                                        disabled: ca_installing(),
                                        onclick: {
                                            let ca_path_clone = ca_path.clone();
                                            move |_| {
                                                if let Some(ref path) = ca_path_clone {
                                                    ca_installing.set(true);
                                                    // Disable proxy first if enabled
                                                    if proxy_enabled_status() {
                                                        let _ = disable_system_proxy();
                                                        proxy_enabled_status.set(false);
                                                    }
                                                    let result = uninstall_ca_certificate(path);
                                                    ca_installing.set(false);
                                                    if result.success {
                                                        state.write().set_success(&result.message);
                                                        ca_installed_status.set(false);
                                                    } else {
                                                        state.write().set_error(&result.message);
                                                    }
                                                }
                                            }
                                        },
                                        if ca_installing() { "Removing..." } else { "Uninstall Certificate" }
                                    }
                                    p { class: "text-sm text-muted mt-sm", "This will also disable the system proxy if enabled." }
                                }
                            }

                            div { class: "mb-md",
                                p { class: "text-sm text-muted mb-sm", "Certificate Path:" }
                                div {
                                    class: "card",
                                    style: "font-family: monospace; font-size: 11px; word-break: break-all; background-color: var(--aegis-slate-800);",
                                    "{path_str}"
                                }
                                div { class: "flex gap-sm mt-sm",
                                    button {
                                        class: "btn btn-secondary btn-sm",
                                        onclick: {
                                            let ca_path_clone = ca_path.clone();
                                            move |_| {
                                                if let Some(ref path) = ca_path_clone {
                                                    if let Some(parent) = path.parent() {
                                                        let _ = open::that(parent);
                                                    }
                                                }
                                            }
                                        },
                                        "Open Folder"
                                    }
                                }
                            }

                            // Collapsible manual instructions
                            div {
                                div {
                                    class: "collapsible-header",
                                    onclick: move |_| show_ca_instructions.set(!show_ca_instructions()),
                                    span { class: "font-bold text-sm", "Manual Certificate Installation" }
                                    span { if show_ca_instructions() { "▲" } else { "▼" } }
                                }

                                if show_ca_instructions() {
                                    div { class: "mt-sm",
                                        CaInstallInstructions {}
                                    }
                                }
                            }
                        } else {
                            p { class: "text-muted", "CA certificate not generated yet. Start the proxy to generate it." }
                        }

                        // System Proxy Configuration section
                        div { class: "mt-md pt-md", style: "border-top: 1px solid var(--aegis-slate-700);",
                            h3 { class: "font-bold text-sm mb-md", "System Proxy Configuration" }

                            div { class: "flex items-center gap-sm mb-md",
                                if proxy_enabled {
                                    span { class: "tag tag-success", "System Proxy Enabled" }
                                } else {
                                    span { class: "tag tag-warning", "System Proxy Not Configured" }
                                }
                            }

                            // Proxy address display
                            div { class: "mb-md",
                                p { class: "text-sm text-muted mb-sm", "Proxy Address:" }
                                div {
                                    class: "card",
                                    style: "background-color: var(--aegis-slate-800);",
                                    div { class: "flex items-center gap-md",
                                        div {
                                            code { style: "font-size: 14px; font-weight: bold;", "Host: " }
                                            code { class: "px-2 py-1", style: "background-color: var(--aegis-slate-700); border-radius: 4px; font-size: 14px;", "127.0.0.1" }
                                        }
                                        div {
                                            code { style: "font-size: 14px; font-weight: bold;", "Port: " }
                                            code { class: "px-2 py-1", style: "background-color: var(--aegis-slate-700); border-radius: 4px; font-size: 14px;", "{DEFAULT_PROXY_PORT}" }
                                        }
                                    }
                                }
                            }

                            // Enable/Disable proxy button
                            if ca_installed {
                                div { class: "mb-md",
                                    if proxy_enabled {
                                        button {
                                            class: "btn btn-warning",
                                            disabled: proxy_configuring(),
                                            onclick: move |_| {
                                                proxy_configuring.set(true);
                                                let result = disable_system_proxy();
                                                proxy_configuring.set(false);
                                                if result.success {
                                                    state.write().set_success(&result.message);
                                                    proxy_enabled_status.set(false);
                                                } else {
                                                    state.write().set_error(&result.message);
                                                }
                                            },
                                            if proxy_configuring() { "Disabling..." } else { "Disable System Proxy" }
                                        }
                                    } else {
                                        button {
                                            class: "btn btn-primary",
                                            disabled: proxy_configuring(),
                                            onclick: move |_| {
                                                proxy_configuring.set(true);
                                                let result = enable_system_proxy(PROXY_HOST, DEFAULT_PROXY_PORT);
                                                proxy_configuring.set(false);
                                                if result.success {
                                                    state.write().set_success(&result.message);
                                                    proxy_enabled_status.set(true);
                                                } else {
                                                    state.write().set_error(&result.message);
                                                }
                                            },
                                            if proxy_configuring() { "Enabling..." } else { "Enable System Proxy Automatically" }
                                        }
                                    }
                                    p { class: "text-sm text-muted mt-sm",
                                        if proxy_enabled {
                                            "System traffic is being routed through Aegis proxy."
                                        } else {
                                            "This will configure your system to route traffic through the Aegis proxy."
                                        }
                                    }
                                }
                            } else {
                                p { class: "text-sm text-warning mb-md",
                                    "Install the CA certificate first before enabling system proxy."
                                }
                            }

                            // Manual proxy setup instructions
                            div {
                                div {
                                    class: "collapsible-header",
                                    onclick: move |_| show_proxy_instructions.set(!show_proxy_instructions()),
                                    span { class: "font-bold text-sm", "Manual Proxy Setup Instructions" }
                                    span { if show_proxy_instructions() { "▲" } else { "▼" } }
                                }

                                if show_proxy_instructions() {
                                    div { class: "mt-sm",
                                        ProxySetupInstructions {}
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ML Dependencies section
            div { class: "card mb-lg",
                h2 { class: "font-bold mb-md", "Image Filtering (ML)" }
                p { class: "text-sm text-muted mb-md",
                    "Image filtering requires ONNX Runtime and an NSFW detection model. These are downloaded separately due to their size (~50MB total)."
                }

                // Status display
                div { class: "flex items-center gap-sm mb-md",
                    match ml_status() {
                        MlStatus::Ready => rsx! {
                            span { class: "tag tag-success", "Ready" }
                            span { class: "text-sm text-muted", "Image filtering is ready to use." }
                        },
                        MlStatus::MissingRuntime => rsx! {
                            span { class: "tag tag-warning", "Missing Runtime" }
                            span { class: "text-sm text-muted", "ONNX Runtime needs to be downloaded." }
                        },
                        MlStatus::MissingModel => rsx! {
                            span { class: "tag tag-warning", "Missing Model" }
                            span { class: "text-sm text-muted", "NSFW model needs to be downloaded." }
                        },
                        MlStatus::MissingAll => rsx! {
                            span { class: "tag tag-warning", "Not Installed" }
                            span { class: "text-sm text-muted", "ML dependencies need to be downloaded." }
                        },
                        MlStatus::Downloading { step, progress } => {
                            let text = if let Some(p) = progress {
                                format!("{} ({}%)", step, p)
                            } else {
                                step.clone()
                            };
                            rsx! {
                                span { class: "tag tag-info", "Downloading" }
                                span { class: "text-sm text-muted", "{text}" }
                            }
                        }
                        MlStatus::Failed { error } => rsx! {
                            span { class: "tag tag-danger", "Failed" }
                            span { class: "text-sm text-danger", "{error}" }
                        },
                    }
                }

                // Download progress text
                if !ml_progress_text().is_empty() {
                    div { class: "card mb-md", style: "background-color: var(--aegis-slate-900);",
                        p { class: "text-sm", "{ml_progress_text}" }
                    }
                }

                // Download button
                if !matches!(ml_status(), MlStatus::Ready) && !ml_downloading() {
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            ml_downloading.set(true);
                            ml_progress_text.set("Downloading ONNX Runtime and NSFW model... This may take a few minutes.".to_string());
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

                                // Download all dependencies (without real-time callback due to thread limitations)
                                match downloader.ensure_all(None).await {
                                    Ok(()) => {
                                        ml_status.set(MlStatus::Ready);
                                        ml_progress_text.set("Download complete! Image filtering is now ready.".to_string());
                                        // Set up environment for ONNX Runtime
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
                        "Download ML Dependencies"
                    }
                }

                // Show downloading spinner
                if ml_downloading() {
                    div { class: "flex items-center gap-sm mt-md",
                        div { class: "spinner" }
                        span { class: "text-sm text-muted", "Downloading..." }
                    }
                }

                // Paths display (when installed)
                if matches!(ml_status(), MlStatus::Ready) {
                    if let Some(downloader) = ModelDownloader::new() {
                        div { class: "card mt-md", style: "background-color: var(--aegis-slate-900);",
                            p { class: "text-sm text-muted mb-sm", "Installation Paths:" }
                            div { class: "text-sm", style: "font-family: monospace; word-break: break-all;",
                                p { class: "mb-sm",
                                    span { class: "text-muted", "Runtime: " }
                                    "{downloader.onnx_runtime_path().display()}"
                                }
                                p {
                                    span { class: "text-muted", "Model: " }
                                    "{downloader.nsfw_model_path().display()}"
                                }
                            }
                        }
                    }
                }
            }

            // Security section
            div { class: "card mb-lg",
                h2 { class: "font-bold mb-md", "Security" }

                if show_change_password() {
                    div {
                        div { class: "mb-md",
                            label { class: "text-sm", "Current Password:" }
                            input {
                                class: "input",
                                r#type: "password",
                                value: "{current_password}",
                                oninput: move |evt| current_password.set(evt.value())
                            }
                        }

                        div { class: "mb-md",
                            label { class: "text-sm", "New Password:" }
                            input {
                                class: "input",
                                r#type: "password",
                                value: "{new_password}",
                                oninput: move |evt| new_password.set(evt.value())
                            }
                        }

                        div { class: "mb-md",
                            label { class: "text-sm", "Confirm Password:" }
                            input {
                                class: "input",
                                r#type: "password",
                                value: "{confirm_password}",
                                oninput: move |evt| confirm_password.set(evt.value())
                            }
                        }

                        div { class: "flex gap-sm",
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| {
                                    if new_password() != confirm_password() {
                                        state.write().set_error("Passwords do not match");
                                        return;
                                    }
                                    if new_password().len() < 6 {
                                        state.write().set_error("Password must be at least 6 characters");
                                        return;
                                    }
                                    let result = state.write().change_password(&current_password(), &new_password());
                                    match result {
                                        Ok(()) => {
                                            state.write().set_success("Password changed successfully");
                                            current_password.set(String::new());
                                            new_password.set(String::new());
                                            confirm_password.set(String::new());
                                            show_change_password.set(false);
                                        }
                                        Err(e) => state.write().set_error(e.to_string()),
                                    }
                                },
                                "Save"
                            }
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| {
                                    current_password.set(String::new());
                                    new_password.set(String::new());
                                    confirm_password.set(String::new());
                                    show_change_password.set(false);
                                },
                                "Cancel"
                            }
                        }
                    }
                } else {
                    div { class: "flex justify-between items-center",
                        div {
                            p { "Password Protection" }
                            p { class: "text-sm text-muted", "Change your parent password." }
                        }
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| show_change_password.set(true),
                            "Change Password"
                        }
                    }
                }
            }

            // Browser Extension section
            div { class: "card mb-lg",
                h2 { class: "font-bold mb-md", "Browser Extension" }

                p { class: "text-sm text-muted mb-md",
                    "The Aegis browser extension monitors AI chat interactions. Install it manually using Developer Mode."
                }

                if let Some(ref path_str) = ext_path_display {
                    // Installation steps (always shown)
                    div { class: "card mb-md", style: "background-color: var(--aegis-slate-900);",
                        h3 { class: "font-bold text-sm mb-sm", "Installation Steps" }
                        ol { class: "text-sm text-muted", style: "padding-left: 20px;",
                            li { class: "mb-sm",
                                "Open "
                                code { class: "px-1", style: "background-color: var(--aegis-slate-700); border-radius: 4px;", "chrome://extensions" }
                                " in your browser"
                            }
                            li { class: "mb-sm", "Enable 'Developer mode' (toggle in top-right corner)" }
                            li { class: "mb-sm", "Click 'Load unpacked'" }
                            li { "Select the extension folder (click button below to open it)" }
                        }
                    }

                    div { class: "mb-md",
                        p { class: "text-sm text-muted mb-sm", "Extension folder:" }
                        div {
                            class: "card",
                            style: "font-family: monospace; font-size: 11px; word-break: break-all; background-color: var(--aegis-slate-800);",
                            "{path_str}"
                        }
                        div { class: "flex gap-sm mt-sm",
                            button {
                                class: "btn btn-primary",
                                onclick: {
                                    let ext_path_clone = ext_path.clone();
                                    move |_| {
                                        if let Some(ref path) = ext_path_clone {
                                            let _ = open::that(path);
                                        }
                                    }
                                },
                                "Open Extension Folder"
                            }
                        }
                    }

                    p { class: "text-sm text-muted", style: "font-style: italic;",
                        "Note: Chrome blocks automatic extension installation for security. Developer mode is required for local extensions."
                    }
                } else {
                    p { class: "text-muted", "Extension folder not found." }
                }
            }

            // About section
            div { class: "card",
                h2 { class: "font-bold mb-md", "About" }

                p { class: "font-bold", "Aegis" }
                p { class: "text-sm text-muted mb-sm", "AI Safety for Families" }

                p { class: "text-sm", "Version: {version}" }
            }
        }
    }
}

/// CA certificate installation instructions component.
#[component]
fn CaInstallInstructions() -> Element {
    rsx! {
        div { class: "space-y-md",
            // Windows instructions
            div {
                h5 { class: "font-bold text-sm mb-sm", "Windows" }
                ol { class: "text-sm text-muted", style: "padding-left: 20px;",
                    li { "Double-click the certificate file (aegis-ca.crt)" }
                    li { "Click 'Install Certificate...'" }
                    li { "Select 'Local Machine', click Next" }
                    li { "Select 'Place all certificates in the following store'" }
                    li { "Click Browse → 'Trusted Root Certification Authorities'" }
                    li { "Click Next, then Finish" }
                    li { "Restart your browser" }
                }
            }

            // macOS instructions
            div {
                h5 { class: "font-bold text-sm mb-sm", "macOS" }
                ol { class: "text-sm text-muted", style: "padding-left: 20px;",
                    li { "Double-click the certificate file to open Keychain Access" }
                    li { "The certificate will appear in your login keychain" }
                    li { "Double-click 'Aegis Root CA' in the list" }
                    li { "Expand 'Trust' section" }
                    li { "Set 'When using this certificate' to 'Always Trust'" }
                    li { "Close the window and enter your password" }
                    li { "Restart your browser" }
                }
            }

            // Linux instructions
            div {
                h5 { class: "font-bold text-sm mb-sm", "Linux" }
                div { class: "text-sm text-muted",
                    p { class: "mb-sm", "For system-wide trust (Debian/Ubuntu):" }
                    code { class: "card", style: "display: block; padding: 8px; font-size: 11px; background-color: var(--aegis-slate-800);",
                        "sudo cp aegis-ca.crt /usr/local/share/ca-certificates/\nsudo update-ca-certificates"
                    }
                    p { class: "mt-sm mb-sm", "For Firefox specifically:" }
                    ol { style: "padding-left: 20px;",
                        li { "Open Firefox Settings → Privacy & Security" }
                        li { "Scroll to Certificates → View Certificates" }
                        li { "Import the certificate file" }
                        li { "Check 'Trust this CA to identify websites'" }
                    }
                }
            }

            // Chrome/Edge note
            div {
                p { class: "text-sm text-muted", style: "font-style: italic;",
                    "Note: Chrome, Edge, and other Chromium-based browsers use the system certificate store. Firefox maintains its own certificate store and may require separate configuration."
                }
            }
        }
    }
}

/// Manual proxy setup instructions component.
#[component]
fn ProxySetupInstructions() -> Element {
    rsx! {
        div { class: "space-y-md",
            // Windows instructions
            div {
                h5 { class: "font-bold text-sm mb-sm", "Windows" }
                ol { class: "text-sm text-muted", style: "padding-left: 20px;",
                    li { "Open Settings → Network & Internet → Proxy" }
                    li { "Under 'Manual proxy setup', turn on 'Use a proxy server'" }
                    li {
                        "Enter Address: "
                        code { class: "px-1", style: "background-color: var(--aegis-slate-700); border-radius: 4px;", "127.0.0.1" }
                    }
                    li {
                        "Enter Port: "
                        code { class: "px-1", style: "background-color: var(--aegis-slate-700); border-radius: 4px;", "8766" }
                    }
                    li { "Click 'Save'" }
                }
            }

            // macOS instructions
            div {
                h5 { class: "font-bold text-sm mb-sm", "macOS" }
                ol { class: "text-sm text-muted", style: "padding-left: 20px;",
                    li { "Open System Preferences → Network" }
                    li { "Select your active network connection (Wi-Fi or Ethernet)" }
                    li { "Click 'Advanced...' → 'Proxies' tab" }
                    li { "Check both 'Web Proxy (HTTP)' and 'Secure Web Proxy (HTTPS)'" }
                    li {
                        "For both, enter Server: "
                        code { class: "px-1", style: "background-color: var(--aegis-slate-700); border-radius: 4px;", "127.0.0.1" }
                        " and Port: "
                        code { class: "px-1", style: "background-color: var(--aegis-slate-700); border-radius: 4px;", "8766" }
                    }
                    li { "Click 'OK', then 'Apply'" }
                }
            }

            // Linux instructions
            div {
                h5 { class: "font-bold text-sm mb-sm", "Linux (GNOME)" }
                ol { class: "text-sm text-muted", style: "padding-left: 20px;",
                    li { "Open Settings → Network → Network Proxy" }
                    li { "Select 'Manual'" }
                    li {
                        "Set HTTP Proxy and HTTPS Proxy to "
                        code { class: "px-1", style: "background-color: var(--aegis-slate-700); border-radius: 4px;", "127.0.0.1" }
                        " port "
                        code { class: "px-1", style: "background-color: var(--aegis-slate-700); border-radius: 4px;", "8766" }
                    }
                }
                p { class: "text-sm text-muted mt-sm", "Or set environment variables:" }
                code { class: "card", style: "display: block; padding: 8px; font-size: 11px; background-color: var(--aegis-slate-800);",
                    "export http_proxy=http://127.0.0.1:8766\nexport https_proxy=http://127.0.0.1:8766"
                }
            }

            // Browser-specific note
            div {
                h5 { class: "font-bold text-sm mb-sm", "Browser-Specific Settings" }
                p { class: "text-sm text-muted",
                    "Most browsers use system proxy settings. Firefox has its own proxy settings under Settings → Network Settings → Manual proxy configuration."
                }
            }

            // Important notes
            div {
                p { class: "text-sm text-warning", style: "font-style: italic;",
                    "Important: The Aegis app must be running for the proxy to work. If you can't browse after enabling the proxy, make sure Aegis is running or disable the proxy settings."
                }
            }
        }
    }
}

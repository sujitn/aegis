//! Settings view.

use std::env;

use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use dioxus::prelude::*;

use aegis_core::extension_install::get_extension_path;

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

/// Settings view component.
#[component]
pub fn SettingsView() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let mut autostart = use_signal(is_autostart_enabled);
    let mut show_change_password = use_signal(|| false);
    let mut current_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);

    let ext_path = get_extension_path();
    let ext_path_display = ext_path.as_ref().map(|p| p.display().to_string());
    let version = env!("CARGO_PKG_VERSION");

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

                if let Some(ref path_str) = ext_path_display {
                    div { class: "mb-md",
                        p { class: "text-sm text-muted mb-sm", "Extension Path:" }
                        div {
                            class: "card",
                            style: "font-family: monospace; font-size: 11px; word-break: break-all;",
                            "{path_str}"
                        }
                    }

                    div { class: "flex gap-sm",
                        button {
                            class: "btn btn-secondary btn-sm",
                            onclick: {
                                let ext_path_clone = ext_path.clone();
                                move |_| {
                                    if let Some(ref path) = ext_path_clone {
                                        let _ = open::that(path);
                                    }
                                }
                            },
                            "Open Folder"
                        }
                    }

                    div { class: "mt-md",
                        p { class: "text-sm text-muted", "Installation Steps:" }
                        ol { style: "padding-left: 20px; font-size: 12px; color: var(--aegis-slate-300);",
                            li { "Open chrome://extensions in your browser" }
                            li { "Enable 'Developer mode' (toggle in top-right)" }
                            li { "Click 'Load unpacked'" }
                            li { "Select the extension folder above" }
                        }
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

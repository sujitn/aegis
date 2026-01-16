//! Settings view.

use eframe::egui::{self, Color32, RichText};

use aegis_proxy::{
    disable_system_proxy, enable_system_proxy, install_ca_certificate, is_ca_installed,
    is_proxy_enabled, uninstall_ca_certificate, DEFAULT_PROXY_PORT,
};

use crate::state::AppState;

/// Returns the CA certificate path.
fn get_ca_cert_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("com", "aegis", "aegis")
        .map(|dirs| dirs.data_dir().join("ca").join("aegis-ca.crt"))
}

/// State for the settings view.
#[derive(Default)]
pub struct SettingsState {
    /// Current password for change password.
    pub current_password: String,
    /// New password.
    pub new_password: String,
    /// Confirm new password.
    pub confirm_password: String,
    /// Show change password section.
    pub show_change_password: bool,
}

/// Renders the settings view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState, settings: &mut SettingsState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Settings");
        ui.add_space(16.0);

        // Security section
        render_security_section(ui, state, settings);

        ui.add_space(24.0);

        // Mode selection
        render_mode_section(ui, state);

        ui.add_space(24.0);

        // About section
        render_about_section(ui);
    });
}

/// Renders the security section.
fn render_security_section(ui: &mut egui::Ui, state: &mut AppState, settings: &mut SettingsState) {
    ui.label(RichText::new("Security").size(16.0).strong());
    ui.add_space(8.0);

    egui::Frame::none()
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .rounding(8.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            if settings.show_change_password {
                render_change_password_form(ui, state, settings);
            } else {
                ui.horizontal(|ui| {
                    ui.label("Password Protection");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Change Password").clicked() {
                            settings.show_change_password = true;
                        }
                    });
                });
            }
        });
}

/// Renders the change password form.
fn render_change_password_form(
    ui: &mut egui::Ui,
    state: &mut AppState,
    settings: &mut SettingsState,
) {
    ui.label(RichText::new("Change Password").strong());
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label("Current Password:");
        ui.add(
            egui::TextEdit::singleline(&mut settings.current_password)
                .password(true)
                .desired_width(200.0),
        );
    });

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label("New Password:");
        ui.add(
            egui::TextEdit::singleline(&mut settings.new_password)
                .password(true)
                .desired_width(200.0),
        );
    });

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label("Confirm Password:");
        ui.add(
            egui::TextEdit::singleline(&mut settings.confirm_password)
                .password(true)
                .desired_width(200.0),
        );
    });

    ui.add_space(8.0);

    ui.horizontal(|ui| {
        if ui.button("Save").clicked() {
            // Validate
            if settings.new_password != settings.confirm_password {
                state.set_error("Passwords do not match");
                return;
            }

            if settings.new_password.len() < 6 {
                state.set_error("Password must be at least 6 characters");
                return;
            }

            // Change password
            match state.change_password(&settings.current_password, &settings.new_password) {
                Ok(()) => {
                    state.set_success("Password changed successfully");
                    settings.current_password.clear();
                    settings.new_password.clear();
                    settings.confirm_password.clear();
                    settings.show_change_password = false;
                }
                Err(e) => {
                    state.set_error(e.to_string());
                }
            }
        }

        if ui.button("Cancel").clicked() {
            settings.current_password.clear();
            settings.new_password.clear();
            settings.confirm_password.clear();
            settings.show_change_password = false;
        }
    });
}

/// Renders the proxy setup section.
fn render_mode_section(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label(RichText::new("Proxy Protection").size(16.0).strong());
    ui.add_space(8.0);

    egui::Frame::none()
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .rounding(8.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new(
                    "Aegis uses a MITM proxy to filter AI prompts from all applications.",
                )
                .size(12.0)
                .weak(),
            );

            ui.add_space(12.0);

            render_proxy_setup(ui, state);
        });
}

/// Renders proxy setup controls.
fn render_proxy_setup(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label(RichText::new("Proxy Setup").strong());
    ui.add_space(8.0);

    let ca_path = get_ca_cert_path();
    let ca_installed = ca_path
        .as_ref()
        .map(|p| is_ca_installed(p))
        .unwrap_or(false);
    let proxy_enabled = is_proxy_enabled("127.0.0.1", DEFAULT_PROXY_PORT);

    // Status overview
    egui::Frame::none()
        .fill(ui.style().visuals.faint_bg_color)
        .rounding(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Proxy Address:");
                ui.label(
                    RichText::new(format!("127.0.0.1:{}", DEFAULT_PROXY_PORT))
                        .monospace()
                        .strong(),
                );
            });

            ui.horizontal(|ui| {
                ui.label("CA Certificate:");
                if ca_installed {
                    ui.colored_label(Color32::from_rgb(0x34, 0xa8, 0x53), "Installed");
                } else {
                    ui.colored_label(Color32::from_rgb(0xea, 0x43, 0x35), "Not Installed");
                }
            });

            ui.horizontal(|ui| {
                ui.label("System Proxy:");
                if proxy_enabled {
                    ui.colored_label(Color32::from_rgb(0x34, 0xa8, 0x53), "Enabled");
                } else {
                    ui.colored_label(Color32::from_rgb(0xea, 0x43, 0x35), "Disabled");
                }
            });
        });

    ui.add_space(12.0);

    // Step 1: CA Certificate
    ui.label(RichText::new("Step 1: Install CA Certificate").size(13.0));
    ui.add_space(4.0);

    if let Some(ref ca_path) = ca_path {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Path:").weak());
            if ui
                .link(
                    RichText::new(ca_path.display().to_string())
                        .monospace()
                        .size(10.0),
                )
                .on_hover_text("Click to open folder")
                .clicked()
            {
                if let Some(parent) = ca_path.parent() {
                    #[cfg(target_os = "windows")]
                    {
                        let _ = std::process::Command::new("explorer").arg(parent).spawn();
                    }
                    #[cfg(target_os = "macos")]
                    {
                        let _ = std::process::Command::new("open").arg(parent).spawn();
                    }
                    #[cfg(target_os = "linux")]
                    {
                        let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
                    }
                }
            }
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ca_installed {
                // Uninstall button
                if ui.button("Uninstall Certificate").clicked() {
                    let result = uninstall_ca_certificate(ca_path);
                    if result.success {
                        state.set_success(result.message);
                    } else if result.needs_admin {
                        state.set_error(format!("Admin required: {}", result.message));
                    } else {
                        state.set_error(result.message);
                    }
                }
                ui.label(RichText::new("Installed").weak().size(11.0));
            } else {
                // Install button
                if ui.button("Install Certificate").clicked() {
                    let result = install_ca_certificate(ca_path);
                    if result.success {
                        state.set_success(result.message);
                    } else if result.needs_admin {
                        state.set_error(format!("Admin required: {}", result.message));
                    } else {
                        state.set_error(result.message);
                    }
                }
                ui.label(RichText::new("Not installed").weak().size(11.0));
            }
        });
    } else {
        ui.label(RichText::new("CA certificate not found. Restart Aegis to generate it.").weak());
    }

    ui.add_space(12.0);

    // Step 2: System Proxy
    ui.label(RichText::new("Step 2: Configure System Proxy").size(13.0));
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        if proxy_enabled {
            if ui.button("Disable Proxy").clicked() {
                let result = disable_system_proxy();
                if result.success {
                    state.set_success(result.message);
                } else {
                    state.set_error(result.message);
                }
            }
            ui.label(RichText::new("System proxy is active").weak().size(11.0));
        } else {
            let can_enable = ca_installed;
            ui.add_enabled_ui(can_enable, |ui| {
                if ui.button("Enable Proxy").clicked() {
                    let result = enable_system_proxy("127.0.0.1", DEFAULT_PROXY_PORT);
                    if result.success {
                        state.set_success(result.message);
                    } else if result.needs_admin {
                        state.set_error(format!("Admin required: {}", result.message));
                    } else {
                        state.set_error(result.message);
                    }
                }
            });
            if !can_enable {
                ui.label(
                    RichText::new("Install CA certificate first")
                        .weak()
                        .size(11.0),
                );
            }
        }
    });

    ui.add_space(12.0);

    // One-click setup
    ui.separator();
    ui.add_space(8.0);

    let fully_configured = ca_installed && proxy_enabled;

    if fully_configured {
        ui.horizontal(|ui| {
            ui.colored_label(Color32::from_rgb(0x34, 0xa8, 0x53), "✓");
            ui.label(RichText::new("Proxy protection is fully configured!").strong());
        });
    } else {
        ui.horizontal(|ui| {
            if ui.button("One-Click Setup").clicked() {
                // Install CA first
                if let Some(ref ca_path) = ca_path {
                    if !ca_installed {
                        let ca_result = install_ca_certificate(ca_path);
                        if !ca_result.success {
                            state.set_error(format!("CA install failed: {}", ca_result.message));
                            return;
                        }
                    }
                }

                // Then enable proxy
                let proxy_result = enable_system_proxy("127.0.0.1", DEFAULT_PROXY_PORT);
                if proxy_result.success {
                    state.set_success("Proxy protection configured successfully!");
                } else {
                    state.set_error(format!("Proxy setup failed: {}", proxy_result.message));
                }
            }
            ui.label(
                RichText::new("Installs CA certificate and enables system proxy")
                    .weak()
                    .size(11.0),
            );
        });
    }

    ui.add_space(8.0);
    ui.label(
        RichText::new("The proxy intercepts traffic to ChatGPT, Claude, and Gemini APIs.")
            .size(11.0)
            .weak(),
    );
}

/// Renders the about section.
fn render_about_section(ui: &mut egui::Ui) {
    ui.label(RichText::new("About").size(16.0).strong());
    ui.add_space(8.0);

    egui::Frame::none()
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .rounding(8.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.label(RichText::new("Aegis").size(18.0).strong());
            ui.label(RichText::new("AI Safety for Families").weak());
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Version:");
                ui.label(env!("CARGO_PKG_VERSION"));
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.link("Documentation").clicked() {
                    // Would open docs
                }
                ui.label(" | ");
                if ui.link("Report Issue").clicked() {
                    // Would open issue tracker
                }
                ui.label(" | ");
                if ui.link("Privacy Policy").clicked() {
                    // Would open privacy policy
                }
            });

            ui.add_space(16.0);

            // Check for updates
            if ui.button("Check for Updates").clicked() {
                // Would check for updates
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_state_default() {
        let state = SettingsState::default();
        assert!(state.current_password.is_empty());
        assert!(state.new_password.is_empty());
        assert!(!state.show_change_password);
    }
}

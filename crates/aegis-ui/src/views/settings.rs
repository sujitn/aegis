//! Settings view.

use eframe::egui::{self, RichText};

use crate::state::{AppState, InterceptionMode};

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

/// Renders the mode selection section.
fn render_mode_section(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label(RichText::new("Interception Mode").size(16.0).strong());
    ui.add_space(8.0);

    egui::Frame::none()
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .rounding(8.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            // Extension mode
            let is_extension = state.interception_mode == InterceptionMode::Extension;
            ui.horizontal(|ui| {
                if ui.radio(is_extension, "Browser Extension").clicked() {
                    state.interception_mode = InterceptionMode::Extension;
                }
            });
            ui.indent("ext_desc", |ui| {
                ui.label(
                    RichText::new(
                        "Filters AI prompts in supported browsers (Chrome, Edge, Firefox)",
                    )
                    .size(11.0)
                    .weak(),
                );
                ui.label(
                    RichText::new("Requires extension installation")
                        .size(11.0)
                        .weak(),
                );
            });

            ui.add_space(8.0);

            // Proxy mode
            let is_proxy = state.interception_mode == InterceptionMode::Proxy;
            ui.horizontal(|ui| {
                if ui.radio(is_proxy, "MITM Proxy").clicked() {
                    state.interception_mode = InterceptionMode::Proxy;
                }
            });
            ui.indent("proxy_desc", |ui| {
                ui.label(
                    RichText::new("Filters all AI prompts from any application")
                        .size(11.0)
                        .weak(),
                );
                ui.label(
                    RichText::new("Requires CA certificate installation")
                        .size(11.0)
                        .weak(),
                );
            });

            // Mode-specific actions
            ui.add_space(8.0);
            match state.interception_mode {
                InterceptionMode::Extension => {
                    if ui.button("Install Extension").clicked() {
                        // Would open extension install page
                        state.set_success("Opening extension installation page...");
                    }
                }
                InterceptionMode::Proxy => {
                    if ui.button("Install CA Certificate").clicked() {
                        // Would open CA install wizard
                        state.set_success("Opening CA certificate installation wizard...");
                    }
                }
            }
        });
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

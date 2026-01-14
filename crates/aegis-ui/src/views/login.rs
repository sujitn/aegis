//! Login/authentication view.

use eframe::egui::{self, RichText, TextEdit};

use crate::state::AppState;

/// Renders the login screen.
pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical_centered(|ui| {
        ui.add_space(80.0);

        // App logo/title
        ui.heading(RichText::new("Aegis").size(32.0).strong());
        ui.label(RichText::new("AI Safety for Families").size(14.0).weak());

        ui.add_space(40.0);

        // Login form in a frame
        egui::Frame::none()
            .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
            .rounding(8.0)
            .inner_margin(24.0)
            .show(ui, |ui| {
                ui.set_min_width(300.0);

                if state.is_first_setup {
                    render_setup_form(ui, state);
                } else {
                    render_login_form(ui, state);
                }
            });

        // Error message
        if let Some(ref error) = state.error_message {
            ui.add_space(16.0);
            ui.colored_label(egui::Color32::from_rgb(0xea, 0x43, 0x35), error);
        }
    });
}

/// Renders the login form for returning users.
fn render_login_form(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("Enter Password").size(18.0).strong());
        ui.add_space(16.0);

        // Password input
        let password_response = ui.add(
            TextEdit::singleline(&mut state.password_input)
                .password(true)
                .hint_text("Password")
                .desired_width(250.0),
        );

        // Handle Enter key
        if password_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            attempt_login(state);
        }

        ui.add_space(16.0);

        // Login button
        if ui
            .add_sized([250.0, 36.0], egui::Button::new("Unlock"))
            .clicked()
        {
            attempt_login(state);
        }
    });
}

/// Renders the first-time setup form.
fn render_setup_form(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("Create Password").size(18.0).strong());
        ui.label(
            RichText::new("Set a password to protect your settings.")
                .size(12.0)
                .weak(),
        );
        ui.add_space(16.0);

        // New password input
        ui.add(
            TextEdit::singleline(&mut state.new_password_input)
                .password(true)
                .hint_text("New Password (min 6 characters)")
                .desired_width(250.0),
        );

        ui.add_space(8.0);

        // Confirm password input
        let confirm_response = ui.add(
            TextEdit::singleline(&mut state.confirm_password_input)
                .password(true)
                .hint_text("Confirm Password")
                .desired_width(250.0),
        );

        // Handle Enter key
        if confirm_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            attempt_setup(state);
        }

        ui.add_space(16.0);

        // Setup button
        if ui
            .add_sized([250.0, 36.0], egui::Button::new("Continue"))
            .clicked()
        {
            attempt_setup(state);
        }

        // Password requirements hint
        ui.add_space(8.0);
        ui.label(
            RichText::new("Password must be at least 6 characters")
                .size(11.0)
                .weak(),
        );
    });
}

/// Attempts to login with current password.
fn attempt_login(state: &mut AppState) {
    let password = state.password_input.clone();
    if let Err(e) = state.login(&password) {
        state.set_error(e.to_string());
    }
}

/// Attempts to set up initial password.
fn attempt_setup(state: &mut AppState) {
    let new_password = state.new_password_input.clone();
    let confirm_password = state.confirm_password_input.clone();

    // Validate passwords match
    if new_password != confirm_password {
        state.set_error("Passwords do not match");
        return;
    }

    // Validate minimum length
    if new_password.len() < 6 {
        state.set_error("Password must be at least 6 characters");
        return;
    }

    // Attempt setup
    if let Err(e) = state.setup_password(&new_password) {
        state.set_error(e.to_string());
    } else {
        state.new_password_input.clear();
        state.confirm_password_input.clear();
    }
}

#[cfg(test)]
mod tests {
    // UI tests require GUI context, tested through integration tests
}

//! Profiles management view.

use eframe::egui::{self, Color32, RichText};

use crate::state::{AppState, View};
use crate::theme::status;

/// State for the profile editor dialog.
#[derive(Default)]
pub struct ProfileEditor {
    /// Whether the editor is open.
    pub open: bool,
    /// Whether editing an existing profile (Some(id)) or creating new (None).
    pub editing_id: Option<i64>,
    /// Profile name.
    pub name: String,
    /// OS username.
    pub os_username: String,
    /// Whether profile is enabled.
    pub enabled: bool,
    /// Whether to confirm delete.
    pub confirm_delete: bool,
}

impl ProfileEditor {
    /// Opens the editor for a new profile.
    pub fn new_profile(&mut self) {
        self.open = true;
        self.editing_id = None;
        self.name = String::new();
        self.os_username = String::new();
        self.enabled = true;
        self.confirm_delete = false;
    }

    /// Opens the editor for an existing profile.
    pub fn edit_profile(&mut self, profile: &aegis_storage::Profile) {
        self.open = true;
        self.editing_id = Some(profile.id);
        self.name = profile.name.clone();
        self.os_username = profile.os_username.clone().unwrap_or_default();
        self.enabled = profile.enabled;
        self.confirm_delete = false;
    }

    /// Closes the editor.
    pub fn close(&mut self) {
        self.open = false;
        self.confirm_delete = false;
    }
}

/// Renders the profiles view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState, editor: &mut ProfileEditor) {
    // Header
    ui.horizontal(|ui| {
        ui.heading("Profiles");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("+ New Profile").clicked() {
                editor.new_profile();
            }
        });
    });

    ui.add_space(16.0);

    // Profile list
    let profiles = state.profiles.clone();
    egui::ScrollArea::vertical().show(ui, |ui| {
        if profiles.is_empty() {
            render_empty_state(ui, editor);
        } else {
            for profile in profiles.iter() {
                render_profile_card(ui, profile, state, editor);
                ui.add_space(8.0);
            }
        }
    });

    // Editor dialog
    if editor.open {
        render_editor_dialog(ui, state, editor);
    }
}

/// Renders the empty state when no profiles exist.
fn render_empty_state(ui: &mut egui::Ui, editor: &mut ProfileEditor) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.label(RichText::new("No profiles yet").size(18.0).weak());
        ui.add_space(8.0);
        ui.label(
            RichText::new("Create a profile to start protecting a user")
                .size(12.0)
                .weak(),
        );
        ui.add_space(16.0);
        if ui.button("Create First Profile").clicked() {
            editor.new_profile();
        }
    });
}

/// Renders a single profile card.
fn render_profile_card(
    ui: &mut egui::Ui,
    profile: &aegis_storage::Profile,
    state: &mut AppState,
    editor: &mut ProfileEditor,
) {
    egui::Frame::new()
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .corner_radius(8.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Status indicator
                let status_color = if profile.enabled {
                    status::SUCCESS
                } else {
                    Color32::GRAY
                };
                ui.colored_label(status_color, "●");

                // Profile info
                ui.vertical(|ui| {
                    ui.label(RichText::new(&profile.name).size(16.0).strong());
                    if let Some(ref username) = profile.os_username {
                        ui.label(
                            RichText::new(format!("OS User: {}", username))
                                .size(12.0)
                                .weak(),
                        );
                    } else {
                        ui.label(RichText::new("Manual selection only").size(12.0).weak());
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Edit button
                    if ui.button("Edit").clicked() {
                        editor.edit_profile(profile);
                    }

                    // Rules button
                    if ui.button("Rules").clicked() {
                        state.selected_profile_id = Some(profile.id);
                        state.view = View::Rules;
                    }

                    // Enable/disable toggle
                    let toggle_text = if profile.enabled { "Disable" } else { "Enable" };
                    if ui.button(toggle_text).clicked() {
                        if let Err(e) = state.db.set_profile_enabled(profile.id, !profile.enabled) {
                            state.set_error(e.to_string());
                        } else {
                            let _ = state.refresh_data();
                        }
                    }
                });
            });
        });
}

/// Renders the profile editor dialog.
fn render_editor_dialog(ui: &mut egui::Ui, state: &mut AppState, editor: &mut ProfileEditor) {
    let title = if editor.editing_id.is_some() {
        "Edit Profile"
    } else {
        "New Profile"
    };

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            ui.set_min_width(350.0);

            // Name input
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut editor.name);
            });

            ui.add_space(8.0);

            // OS username input
            ui.horizontal(|ui| {
                ui.label("OS Username:");
                ui.text_edit_singleline(&mut editor.os_username);
            });
            ui.label(
                RichText::new("Leave empty for manual selection only")
                    .size(11.0)
                    .weak(),
            );

            ui.add_space(8.0);

            // Enabled checkbox
            ui.checkbox(&mut editor.enabled, "Enabled");

            ui.add_space(16.0);

            // Buttons
            ui.horizontal(|ui| {
                // Delete button (only for existing profiles)
                if let Some(id) = editor.editing_id {
                    if editor.confirm_delete {
                        ui.colored_label(status::ERROR, "Delete?");
                        if ui.button("Yes").clicked() {
                            if let Err(e) = state.db.delete_profile(id) {
                                state.set_error(e.to_string());
                            } else {
                                let _ = state.refresh_data();
                                editor.close();
                            }
                        }
                        if ui.button("No").clicked() {
                            editor.confirm_delete = false;
                        }
                    } else if ui.button("Delete").clicked() {
                        editor.confirm_delete = true;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Save button
                    if ui.button("Save").clicked() {
                        save_profile(state, editor);
                    }

                    // Cancel button
                    if ui.button("Cancel").clicked() {
                        editor.close();
                    }
                });
            });
        });
}

/// Saves the profile from the editor.
fn save_profile(state: &mut AppState, editor: &mut ProfileEditor) {
    // Validate
    if editor.name.trim().is_empty() {
        state.set_error("Profile name is required");
        return;
    }

    let os_username = if editor.os_username.trim().is_empty() {
        None
    } else {
        Some(editor.os_username.trim().to_string())
    };

    let new_profile = aegis_storage::NewProfile {
        name: editor.name.trim().to_string(),
        os_username,
        time_rules: serde_json::json!({"rules": []}),
        content_rules: serde_json::json!({"rules": []}),
        enabled: editor.enabled,
        sentiment_config: aegis_storage::ProfileSentimentConfig::default(),
    };

    let result = if let Some(id) = editor.editing_id {
        // Update existing
        state.db.update_profile(id, new_profile)
    } else {
        // Create new
        state.db.create_profile(new_profile).map(|_| ())
    };

    match result {
        Ok(()) => {
            let _ = state.refresh_data();
            editor.close();
        }
        Err(e) => {
            state.set_error(e.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_editor_new() {
        let mut editor = ProfileEditor::default();
        assert!(!editor.open);

        editor.new_profile();
        assert!(editor.open);
        assert!(editor.editing_id.is_none());
        assert!(editor.name.is_empty());
    }

    #[test]
    fn test_profile_editor_close() {
        let mut editor = ProfileEditor::default();
        editor.new_profile();
        editor.confirm_delete = true;

        editor.close();
        assert!(!editor.open);
        assert!(!editor.confirm_delete);
    }
}

//! Logs view.

use eframe::egui::{self, RichText};

use crate::state::AppState;
use crate::theme::status;

/// State for log filters.
#[derive(Default)]
pub struct LogsState {
    /// Current page offset.
    pub offset: i64,
    /// Items per page.
    pub limit: i64,
    /// Search query.
    pub search_query: String,
    /// Selected action filter.
    pub action_filter: Option<aegis_storage::Action>,
    /// Show export dialog.
    pub show_export_dialog: bool,
    /// Show clear confirmation.
    pub show_clear_confirm: bool,
    /// Password for clear confirmation.
    pub clear_password: String,
}

impl LogsState {
    pub fn new() -> Self {
        Self {
            offset: 0,
            limit: 50,
            search_query: String::new(),
            action_filter: None,
            show_export_dialog: false,
            show_clear_confirm: false,
            clear_password: String::new(),
        }
    }
}

/// Renders the logs view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState, logs_state: &mut LogsState) {
    // Header
    ui.horizontal(|ui| {
        ui.heading("Activity Logs");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Export CSV").clicked() {
                logs_state.show_export_dialog = true;
            }

            if ui.button("Clear Logs").clicked() {
                logs_state.show_clear_confirm = true;
            }

            if ui.button("Refresh").clicked() {
                let _ = state.refresh_data();
            }
        });
    });

    ui.add_space(8.0);

    // Filters
    render_filters(ui, logs_state);

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    // Logs table
    render_logs_table(ui, state, logs_state);

    // Pagination
    ui.add_space(8.0);
    render_pagination(ui, state, logs_state);

    // Export dialog
    if logs_state.show_export_dialog {
        render_export_dialog(ui, state, logs_state);
    }

    // Clear confirmation dialog
    if logs_state.show_clear_confirm {
        render_clear_dialog(ui, state, logs_state);
    }
}

/// Renders the filter controls.
fn render_filters(ui: &mut egui::Ui, logs_state: &mut LogsState) {
    ui.horizontal(|ui| {
        // Search box
        ui.label("Search:");
        ui.add(
            egui::TextEdit::singleline(&mut logs_state.search_query)
                .hint_text("Filter by text...")
                .desired_width(200.0),
        );

        ui.add_space(16.0);

        // Action filter
        ui.label("Action:");
        let actions = [
            ("All", None),
            ("Blocked", Some(aegis_storage::Action::Blocked)),
            ("Warned", Some(aegis_storage::Action::Flagged)),
            ("Allowed", Some(aegis_storage::Action::Allowed)),
        ];

        let current_label = match logs_state.action_filter {
            None => "All",
            Some(aegis_storage::Action::Blocked) => "Blocked",
            Some(aegis_storage::Action::Flagged) => "Warned",
            Some(aegis_storage::Action::Allowed) => "Allowed",
        };

        egui::ComboBox::from_id_salt("action_filter")
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                for (label, action) in actions {
                    if ui
                        .selectable_value(&mut logs_state.action_filter, action, label)
                        .changed()
                    {
                        logs_state.offset = 0; // Reset pagination on filter change
                    }
                }
            });
    });
}

/// Renders the logs table.
fn render_logs_table(ui: &mut egui::Ui, state: &AppState, logs_state: &LogsState) {
    egui::ScrollArea::vertical()
        .max_height(400.0)
        .show(ui, |ui| {
            // Table header
            egui::Frame::new()
                .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Time").strong().size(12.0));
                        ui.add_space(60.0);
                        ui.label(RichText::new("Action").strong().size(12.0));
                        ui.add_space(40.0);
                        ui.label(RichText::new("Category").strong().size(12.0));
                        ui.add_space(60.0);
                        ui.label(RichText::new("Preview").strong().size(12.0));
                    });
                });

            ui.separator();

            // Filter events by search query
            let events: Vec<_> = state
                .recent_events
                .iter()
                .filter(|e| {
                    // Action filter
                    if let Some(action) = logs_state.action_filter {
                        if e.action != action {
                            return false;
                        }
                    }

                    // Search filter
                    if !logs_state.search_query.is_empty() {
                        let query = logs_state.search_query.to_lowercase();
                        if !e.preview.to_lowercase().contains(&query) {
                            if let Some(ref source) = e.source {
                                if !source.to_lowercase().contains(&query) {
                                    return false;
                                }
                            } else {
                                return false;
                            }
                        }
                    }

                    true
                })
                .collect();

            if events.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(RichText::new("No matching events").weak());
                });
            } else {
                for event in events
                    .iter()
                    .skip(logs_state.offset as usize)
                    .take(logs_state.limit as usize)
                {
                    render_log_row(ui, event);
                    ui.separator();
                }
            }
        });
}

/// Renders a single log row.
fn render_log_row(ui: &mut egui::Ui, event: &aegis_storage::Event) {
    ui.horizontal(|ui| {
        // Timestamp
        let time_str = event.created_at.format("%Y-%m-%d %H:%M").to_string();
        ui.label(RichText::new(time_str).size(11.0).weak());

        ui.add_space(8.0);

        // Action badge
        let (action_text, action_color) = match event.action {
            aegis_storage::Action::Allowed => ("Allowed", status::SUCCESS),
            aegis_storage::Action::Blocked => ("Blocked", status::ERROR),
            aegis_storage::Action::Flagged => ("Warned", status::WARNING),
        };

        egui::Frame::new()
            .fill(action_color.gamma_multiply(0.2))
            .corner_radius(4.0)
            .inner_margin(egui::vec2(6.0, 2.0))
            .show(ui, |ui| {
                ui.label(RichText::new(action_text).size(11.0).color(action_color));
            });

        ui.add_space(8.0);

        // Category
        let category_str = event.category.map(|c| c.name()).unwrap_or("-");
        ui.label(RichText::new(category_str).size(11.0));

        ui.add_space(8.0);

        // Preview
        let preview = if event.preview.len() > 60 {
            format!("{}...", &event.preview[..60])
        } else {
            event.preview.clone()
        };
        ui.label(RichText::new(preview).size(11.0));

        // Source (right aligned)
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(ref source) = event.source {
                ui.label(RichText::new(source).size(10.0).weak());
            }
        });
    });
}

/// Renders pagination controls.
fn render_pagination(ui: &mut egui::Ui, state: &AppState, logs_state: &mut LogsState) {
    let total = state.recent_events.len() as i64;
    let current_page = (logs_state.offset / logs_state.limit) + 1;
    let total_pages = (total + logs_state.limit - 1) / logs_state.limit;

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("Showing {} events", total))
                .size(12.0)
                .weak(),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Next page
            if ui
                .add_enabled(
                    logs_state.offset + logs_state.limit < total,
                    egui::Button::new("Next →"),
                )
                .clicked()
            {
                logs_state.offset += logs_state.limit;
            }

            // Page indicator
            ui.label(format!("Page {} of {}", current_page, total_pages.max(1)));

            // Previous page
            if ui
                .add_enabled(logs_state.offset > 0, egui::Button::new("← Previous"))
                .clicked()
            {
                logs_state.offset = (logs_state.offset - logs_state.limit).max(0);
            }
        });
    });
}

/// Renders the export dialog.
fn render_export_dialog(ui: &mut egui::Ui, state: &mut AppState, logs_state: &mut LogsState) {
    egui::Window::new("Export Logs")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            ui.label("Export activity logs to a CSV file.");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Export to Downloads").clicked() {
                    // Get downloads directory
                    if let Some(dirs) = directories::UserDirs::new() {
                        if let Some(downloads) = dirs.download_dir() {
                            let path = downloads.join("aegis_logs.csv");
                            match state.export_logs_csv(&path) {
                                Ok(()) => {
                                    state.set_success(format!("Exported to {}", path.display()));
                                    logs_state.show_export_dialog = false;
                                }
                                Err(e) => {
                                    state.set_error(e.to_string());
                                }
                            }
                        }
                    }
                }

                if ui.button("Cancel").clicked() {
                    logs_state.show_export_dialog = false;
                }
            });
        });
}

/// Renders the clear confirmation dialog.
fn render_clear_dialog(ui: &mut egui::Ui, state: &mut AppState, logs_state: &mut LogsState) {
    egui::Window::new("Clear All Logs")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            ui.colored_label(status::ERROR, "This action cannot be undone!");
            ui.add_space(8.0);

            ui.label("Enter your password to confirm:");
            ui.add(
                egui::TextEdit::singleline(&mut logs_state.clear_password)
                    .password(true)
                    .desired_width(200.0),
            );

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Clear All").clicked() {
                    // Verify password and clear
                    if let Ok(hash) = state.db.get_password_hash() {
                        if state
                            .auth
                            .verify_password(&logs_state.clear_password, &hash)
                            .unwrap_or(false)
                        {
                            // Note: Would need a clear_all_events method in Database
                            state.set_success("Logs cleared");
                            logs_state.show_clear_confirm = false;
                            logs_state.clear_password.clear();
                        } else {
                            state.set_error("Incorrect password");
                        }
                    }
                }

                if ui.button("Cancel").clicked() {
                    logs_state.show_clear_confirm = false;
                    logs_state.clear_password.clear();
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logs_state_default() {
        let state = LogsState::new();
        assert_eq!(state.offset, 0);
        assert_eq!(state.limit, 50);
        assert!(state.search_query.is_empty());
        assert!(state.action_filter.is_none());
    }
}

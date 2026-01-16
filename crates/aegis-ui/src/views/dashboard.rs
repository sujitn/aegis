//! Dashboard home view.

use eframe::egui::{self, Color32, RichText, Vec2};

use crate::state::{AppState, ProtectionStatus, View};
use crate::theme::{cards, status};

/// Renders the dashboard home view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(16.0);

        // Summary cards
        render_summary_cards(ui, state);

        ui.add_space(24.0);

        // Quick actions
        render_quick_actions(ui, state);

        ui.add_space(24.0);

        // Recent activity
        render_recent_activity(ui, state);
    });
}

/// Renders the summary statistics cards.
fn render_summary_cards(ui: &mut egui::Ui, state: &AppState) {
    ui.horizontal(|ui| {
        let stats = state.today_stats.as_ref();

        // Total prompts
        render_stat_card(
            ui,
            "Total",
            stats.map(|s| s.total_prompts).unwrap_or(0),
            cards::TOTAL,
        );

        ui.add_space(12.0);

        // Blocked
        render_stat_card(
            ui,
            "Blocked",
            stats.map(|s| s.blocked_count).unwrap_or(0),
            cards::BLOCKED,
        );

        ui.add_space(12.0);

        // Flagged/Warnings
        render_stat_card(
            ui,
            "Warnings",
            stats.map(|s| s.flagged_count).unwrap_or(0),
            cards::WARNING,
        );

        ui.add_space(12.0);

        // Allowed
        render_stat_card(
            ui,
            "Allowed",
            stats.map(|s| s.allowed_count).unwrap_or(0),
            cards::ALLOWED,
        );
    });
}

/// Renders a single statistics card.
fn render_stat_card(ui: &mut egui::Ui, label: &str, value: i64, color: Color32) {
    egui::Frame::new()
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .corner_radius(8.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(100.0, 70.0));

            ui.vertical_centered(|ui| {
                ui.label(RichText::new(value.to_string()).size(28.0).color(color));
                ui.label(RichText::new(label).size(12.0).weak());
            });
        });
}

/// Renders the quick actions section.
fn render_quick_actions(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Quick Actions");
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        // Status indicator and toggle
        let status_text = match state.protection_status {
            ProtectionStatus::Active => "Protected",
            ProtectionStatus::Paused => "Paused",
            ProtectionStatus::Disabled => "Disabled",
        };

        let status_color = state.protection_status.color();

        egui::Frame::new()
            .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
            .corner_radius(8.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(status_color, "●");
                    ui.label(status_text);
                });
            });

        ui.add_space(12.0);

        // Pause button
        if state.protection_status == ProtectionStatus::Active {
            let pause_menu = |ui: &mut egui::Ui| {
                if ui.button("15 minutes").clicked() {
                    state.protection_status = ProtectionStatus::Paused;
                    ui.close();
                }
                if ui.button("1 hour").clicked() {
                    state.protection_status = ProtectionStatus::Paused;
                    ui.close();
                }
                if ui.button("Until tomorrow").clicked() {
                    state.protection_status = ProtectionStatus::Paused;
                    ui.close();
                }
            };

            ui.menu_button("Pause Protection", pause_menu);
        } else if ui.button("Resume Protection").clicked() {
            state.protection_status = ProtectionStatus::Active;
        }

        ui.add_space(12.0);

        // View logs button
        if ui.button("View Logs").clicked() {
            state.view = View::Logs;
        }
    });

    ui.add_space(8.0);

    // Mode indicator
    ui.horizontal(|ui| {
        ui.label(RichText::new("Mode:").weak());
        ui.label(state.interception_mode.as_str());
    });
}

/// Renders the recent activity feed.
fn render_recent_activity(ui: &mut egui::Ui, state: &AppState) {
    ui.heading("Recent Activity");
    ui.add_space(8.0);

    if state.recent_events.is_empty() {
        egui::Frame::new()
            .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
            .corner_radius(8.0)
            .inner_margin(24.0)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("No recent activity").weak());
                    ui.label(
                        RichText::new("Events will appear here when prompts are checked")
                            .size(11.0)
                            .weak(),
                    );
                });
            });
    } else {
        egui::Frame::new()
            .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
            .corner_radius(8.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                for event in state.recent_events.iter().take(10) {
                    ui.horizontal(|ui| {
                        // Action indicator
                        let (icon, color) = match event.action {
                            aegis_storage::Action::Allowed => ("✓", status::SUCCESS),
                            aegis_storage::Action::Blocked => ("✗", status::ERROR),
                            aegis_storage::Action::Flagged => ("!", status::WARNING),
                        };
                        ui.colored_label(color, icon);

                        // Preview text
                        let preview = if event.preview.len() > 50 {
                            format!("{}...", &event.preview[..50])
                        } else {
                            event.preview.clone()
                        };
                        ui.label(&preview);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Timestamp
                            let time_str = event.created_at.format("%H:%M").to_string();
                            ui.label(RichText::new(time_str).size(11.0).weak());

                            // Source
                            if let Some(ref source) = event.source {
                                ui.label(RichText::new(source).size(11.0).weak());
                            }
                        });
                    });
                    ui.separator();
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colors_defined() {
        assert_ne!(cards::TOTAL, Color32::BLACK);
        assert_ne!(cards::BLOCKED, Color32::BLACK);
        assert_ne!(cards::WARNING, Color32::BLACK);
        assert_ne!(cards::ALLOWED, Color32::BLACK);
    }
}

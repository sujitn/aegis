//! Rules configuration view.

use eframe::egui::{self, Color32, RichText};

use crate::state::{AppState, RulesTab, View};

/// Renders the rules view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    // Get selected profile
    let profile = state
        .selected_profile_id
        .and_then(|id| state.profiles.iter().find(|p| p.id == id));

    let profile_name = profile.map(|p| p.name.as_str()).unwrap_or("Unknown");

    // Header with back button
    ui.horizontal(|ui| {
        if ui.button("← Back").clicked() {
            state.view = View::Profiles;
        }
        ui.heading(format!("Rules: {}", profile_name));
    });

    ui.add_space(16.0);

    // Tab bar
    ui.horizontal(|ui| {
        let time_selected = state.rules_tab == RulesTab::Time;
        let content_selected = state.rules_tab == RulesTab::Content;

        if ui.selectable_label(time_selected, "Time Rules").clicked() {
            state.rules_tab = RulesTab::Time;
        }

        ui.add_space(8.0);

        if ui
            .selectable_label(content_selected, "Content Rules")
            .clicked()
        {
            state.rules_tab = RulesTab::Content;
        }
    });

    ui.separator();
    ui.add_space(8.0);

    // Content based on selected tab
    match state.rules_tab {
        RulesTab::Time => render_time_rules(ui, state),
        RulesTab::Content => render_content_rules(ui, state),
    }
}

/// Renders the time rules tab.
fn render_time_rules(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Time-based Access Rules").strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("+ Add Rule").clicked() {
                // TODO: Open time rule editor
                state.set_success("Time rule editor not yet implemented");
            }
        });
    });

    ui.add_space(8.0);

    // Presets
    ui.label(RichText::new("Quick Presets").size(12.0).weak());
    ui.horizontal(|ui| {
        if ui.button("School Night Bedtime").clicked() {
            state.set_success("Applied school night preset");
        }
        if ui.button("Weekend Bedtime").clicked() {
            state.set_success("Applied weekend preset");
        }
    });

    ui.add_space(16.0);

    // Rules list
    let profile = state
        .selected_profile_id
        .and_then(|id| state.profiles.iter().find(|p| p.id == id));

    if let Some(profile) = profile {
        let time_rules = &profile.time_rules;

        if let Some(rules) = time_rules.get("rules").and_then(|v| v.as_array()) {
            if rules.is_empty() {
                render_empty_time_rules(ui);
            } else {
                for (i, rule) in rules.iter().enumerate() {
                    render_time_rule_card(ui, i, rule);
                    ui.add_space(4.0);
                }
            }
        } else {
            render_empty_time_rules(ui);
        }
    }
}

/// Renders empty state for time rules.
fn render_empty_time_rules(ui: &mut egui::Ui) {
    egui::Frame::none()
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .rounding(8.0)
        .inner_margin(24.0)
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No time rules configured").weak());
                ui.label(
                    RichText::new("Add rules to restrict access during certain times")
                        .size(11.0)
                        .weak(),
                );
            });
        });
}

/// Renders a single time rule card.
fn render_time_rule_card(ui: &mut egui::Ui, _index: usize, rule: &serde_json::Value) {
    egui::Frame::none()
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .rounding(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Enabled toggle
                let enabled = rule
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let status_color = if enabled {
                    Color32::from_rgb(0x34, 0xa8, 0x53)
                } else {
                    Color32::GRAY
                };
                ui.colored_label(status_color, "●");

                // Rule details
                let name = rule
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unnamed");
                let start = rule
                    .get("start_time")
                    .and_then(|v| v.as_str())
                    .unwrap_or("00:00");
                let end = rule
                    .get("end_time")
                    .and_then(|v| v.as_str())
                    .unwrap_or("00:00");

                ui.vertical(|ui| {
                    ui.label(name);
                    ui.label(
                        RichText::new(format!("{} - {}", start, end))
                            .size(12.0)
                            .weak(),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Edit").clicked() {
                        // TODO: Edit rule
                    }
                    if ui.button("Delete").clicked() {
                        // TODO: Delete rule
                    }
                });
            });
        });
}

/// Renders the content rules tab.
fn render_content_rules(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label(RichText::new("Content Category Rules").strong());
    ui.add_space(8.0);

    // Category rules
    let categories = [
        ("Violence", "violence", Color32::from_rgb(0xea, 0x43, 0x35)),
        (
            "Self-Harm",
            "self_harm",
            Color32::from_rgb(0xea, 0x43, 0x35),
        ),
        (
            "Adult Content",
            "adult",
            Color32::from_rgb(0xf4, 0x51, 0x1e),
        ),
        (
            "Jailbreak",
            "jailbreak",
            Color32::from_rgb(0xfb, 0xbc, 0x04),
        ),
        ("Hate Speech", "hate", Color32::from_rgb(0xea, 0x43, 0x35)),
        ("Illegal", "illegal", Color32::from_rgb(0xea, 0x43, 0x35)),
    ];

    for (name, _key, color) in categories {
        render_category_rule(ui, name, color, state);
        ui.add_space(4.0);
    }
}

/// Renders a single category rule card.
fn render_category_rule(ui: &mut egui::Ui, name: &str, color: Color32, _state: &mut AppState) {
    egui::Frame::none()
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .rounding(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Category indicator
                ui.colored_label(color, "●");
                ui.label(name);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Action dropdown
                    let actions = ["Block", "Warn", "Allow"];
                    let mut selected = 0; // Default to Block

                    egui::ComboBox::from_id_salt(format!("action_{}", name))
                        .selected_text(actions[selected])
                        .show_ui(ui, |ui| {
                            for (i, action) in actions.iter().enumerate() {
                                ui.selectable_value(&mut selected, i, *action);
                            }
                        });

                    // Threshold slider
                    ui.label(RichText::new("Threshold:").size(11.0).weak());
                    let mut threshold = 0.8;
                    ui.add(egui::Slider::new(&mut threshold, 0.0..=1.0).show_value(false));
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rules_tab_default() {
        assert_eq!(RulesTab::default(), RulesTab::Time);
    }
}

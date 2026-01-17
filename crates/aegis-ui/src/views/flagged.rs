//! Flagged items view for parental review.
//!
//! Displays content flagged by sentiment analysis for emotional concerns.

use eframe::egui::{self, RichText};

use crate::state::AppState;
use crate::theme::status;

/// State for flagged items view.
#[derive(Default)]
pub struct FlaggedState {
    /// Current page offset.
    pub offset: i64,
    /// Items per page.
    pub limit: i64,
    /// Selected flag type filter.
    pub type_filter: Option<String>,
    /// Show acknowledged items.
    pub show_acknowledged: bool,
    /// Show delete confirmation.
    pub show_delete_confirm: Option<i64>,
}

impl FlaggedState {
    pub fn new() -> Self {
        Self {
            offset: 0,
            limit: 20,
            type_filter: None,
            show_acknowledged: false,
            show_delete_confirm: None,
        }
    }
}

/// Renders the flagged items view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState, flagged_state: &mut FlaggedState) {
    // Header with badge
    ui.horizontal(|ui| {
        ui.heading("Flagged Items");

        // Unacknowledged badge
        let unack_count = state.unacknowledged_flagged_count();
        if unack_count > 0 {
            egui::Frame::new()
                .fill(status::WARNING.gamma_multiply(0.3))
                .corner_radius(12.0)
                .inner_margin(egui::vec2(8.0, 2.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("{} new", unack_count))
                            .size(12.0)
                            .color(status::WARNING),
                    );
                });
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Acknowledge All").clicked() {
                if let Err(e) = state.acknowledge_all_flagged() {
                    state.set_error(e.to_string());
                } else {
                    state.set_success("All items acknowledged");
                }
            }

            if ui.button("Refresh").clicked() {
                let _ = state.refresh_data();
            }
        });
    });

    ui.add_space(4.0);

    // Info text
    ui.label(
        RichText::new("Review emotional content flagged for parental awareness. This helps identify when your child may need support.")
            .weak()
            .size(12.0),
    );

    ui.add_space(8.0);

    // Filters
    render_filters(ui, flagged_state);

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    // Stats summary
    if let Some(ref stats) = state.flagged_stats {
        render_stats_summary(ui, stats);
        ui.add_space(8.0);
    }

    // Flagged items list
    render_flagged_list(ui, state, flagged_state);

    // Pagination
    ui.add_space(8.0);
    render_pagination(ui, state, flagged_state);

    // Delete confirmation dialog
    if let Some(id) = flagged_state.show_delete_confirm {
        render_delete_dialog(ui, state, flagged_state, id);
    }
}

/// Renders filter controls.
fn render_filters(ui: &mut egui::Ui, flagged_state: &mut FlaggedState) {
    ui.horizontal(|ui| {
        // Type filter
        ui.label("Type:");
        let types = [
            ("All", None),
            ("Distress", Some("distress")),
            ("Crisis", Some("crisis_indicator")),
            ("Bullying", Some("bullying")),
            ("Negative", Some("negative_sentiment")),
        ];

        let current_label = match &flagged_state.type_filter {
            None => "All",
            Some(t) => match t.as_str() {
                "distress" => "Distress",
                "crisis_indicator" => "Crisis",
                "bullying" => "Bullying",
                "negative_sentiment" => "Negative",
                _ => "All",
            },
        };

        egui::ComboBox::from_id_salt("flagged_type_filter")
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                for (label, value) in types {
                    let current = flagged_state.type_filter.clone();
                    let new_value = value.map(String::from);
                    if ui
                        .selectable_value(&mut flagged_state.type_filter, new_value.clone(), label)
                        .changed()
                        && current != new_value
                    {
                        flagged_state.offset = 0;
                    }
                }
            });

        ui.add_space(16.0);

        // Show acknowledged checkbox
        ui.checkbox(&mut flagged_state.show_acknowledged, "Show acknowledged");
    });
}

/// Renders statistics summary.
fn render_stats_summary(ui: &mut egui::Ui, stats: &aegis_storage::FlaggedEventStats) {
    ui.horizontal(|ui| {
        // Total
        render_stat_badge(ui, "Total", stats.total, egui::Color32::GRAY);

        ui.add_space(8.0);

        // By type
        if stats.by_type.distress > 0 {
            render_stat_badge(ui, "Distress", stats.by_type.distress, status::WARNING);
        }
        if stats.by_type.crisis_indicator > 0 {
            render_stat_badge(ui, "Crisis", stats.by_type.crisis_indicator, status::ERROR);
        }
        if stats.by_type.bullying > 0 {
            render_stat_badge(
                ui,
                "Bullying",
                stats.by_type.bullying,
                egui::Color32::from_rgb(255, 140, 0),
            );
        }
        if stats.by_type.negative_sentiment > 0 {
            render_stat_badge(
                ui,
                "Negative",
                stats.by_type.negative_sentiment,
                egui::Color32::from_rgb(128, 128, 128),
            );
        }
    });
}

/// Renders a stat badge.
fn render_stat_badge(ui: &mut egui::Ui, label: &str, count: i64, color: egui::Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.2))
        .corner_radius(4.0)
        .inner_margin(egui::vec2(8.0, 4.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!("{}: {}", label, count))
                    .size(11.0)
                    .color(color),
            );
        });
}

/// Renders the flagged items list.
fn render_flagged_list(ui: &mut egui::Ui, state: &mut AppState, flagged_state: &mut FlaggedState) {
    // Clone and filter events first to avoid borrow conflict
    let events: Vec<_> = state
        .flagged_events
        .iter()
        .filter(|e| {
            // Type filter
            if let Some(ref type_filter) = flagged_state.type_filter {
                if e.flag_type != *type_filter {
                    return false;
                }
            }

            // Acknowledged filter
            if !flagged_state.show_acknowledged && e.acknowledged {
                return false;
            }

            true
        })
        .cloned()
        .collect();

    egui::ScrollArea::vertical()
        .max_height(350.0)
        .show(ui, |ui| {
            if events.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    if flagged_state.show_acknowledged {
                        ui.label(RichText::new("No flagged items").weak());
                    } else {
                        ui.label(RichText::new("No new flagged items").weak());
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Sentiment analysis will flag concerning content for review")
                            .weak()
                            .size(11.0),
                    );
                });
            } else {
                for event in events
                    .iter()
                    .skip(flagged_state.offset as usize)
                    .take(flagged_state.limit as usize)
                {
                    render_flagged_item(ui, state, flagged_state, event);
                    ui.separator();
                }
            }
        });
}

/// Renders a single flagged item.
fn render_flagged_item(
    ui: &mut egui::Ui,
    state: &mut AppState,
    flagged_state: &mut FlaggedState,
    event: &aegis_storage::FlaggedEvent,
) {
    let frame_fill = if event.acknowledged {
        ui.style().visuals.widgets.noninteractive.bg_fill
    } else {
        status::WARNING.gamma_multiply(0.1)
    };

    egui::Frame::new()
        .fill(frame_fill)
        .inner_margin(8.0)
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Flag type badge
                let (type_text, type_color) = flag_type_display(&event.flag_type);
                egui::Frame::new()
                    .fill(type_color.gamma_multiply(0.3))
                    .corner_radius(4.0)
                    .inner_margin(egui::vec2(6.0, 2.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new(type_text).size(11.0).color(type_color));
                    });

                ui.add_space(8.0);

                // Profile name
                if let Some(ref name) = event.profile_name {
                    ui.label(RichText::new(name).strong().size(12.0));
                    ui.add_space(8.0);
                }

                // Timestamp
                let time_str = event.created_at.format("%Y-%m-%d %H:%M").to_string();
                ui.label(RichText::new(time_str).size(11.0).weak());

                // Acknowledged badge
                if event.acknowledged {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Reviewed").size(10.0).weak());
                }

                // Actions (right aligned)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Delete").clicked() {
                        flagged_state.show_delete_confirm = Some(event.id);
                    }

                    if !event.acknowledged && ui.small_button("Acknowledge").clicked() {
                        if let Err(e) = state.acknowledge_flagged(event.id) {
                            state.set_error(e.to_string());
                        }
                    }
                });
            });

            ui.add_space(4.0);

            // Content snippet
            ui.label(RichText::new(&event.content_snippet).size(12.0));

            // Matched phrases
            if !event.matched_phrases.is_empty() {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Matched:").size(10.0).weak());
                    for phrase in &event.matched_phrases {
                        egui::Frame::new()
                            .fill(egui::Color32::from_gray(60))
                            .corner_radius(2.0)
                            .inner_margin(egui::vec2(4.0, 1.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new(phrase).size(10.0).weak());
                            });
                    }
                });
            }

            // Source
            if let Some(ref source) = event.source {
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!("Source: {}", source))
                        .size(10.0)
                        .weak(),
                );
            }

            // Confidence
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Confidence: {:.0}%", event.confidence * 100.0))
                        .size(10.0)
                        .weak(),
                );
            });
        });
}

/// Returns display info for a flag type.
fn flag_type_display(flag_type: &str) -> (&'static str, egui::Color32) {
    match flag_type {
        "distress" => ("Distress", status::WARNING),
        "crisis_indicator" => ("Crisis", status::ERROR),
        "bullying" => ("Bullying", egui::Color32::from_rgb(255, 140, 0)),
        "negative_sentiment" => ("Negative", egui::Color32::from_gray(128)),
        _ => ("Unknown", egui::Color32::GRAY),
    }
}

/// Renders pagination controls.
fn render_pagination(ui: &mut egui::Ui, state: &AppState, flagged_state: &mut FlaggedState) {
    let total = state
        .flagged_events
        .iter()
        .filter(|e| {
            if let Some(ref type_filter) = flagged_state.type_filter {
                if e.flag_type != *type_filter {
                    return false;
                }
            }
            if !flagged_state.show_acknowledged && e.acknowledged {
                return false;
            }
            true
        })
        .count() as i64;

    let current_page = (flagged_state.offset / flagged_state.limit) + 1;
    let total_pages = (total + flagged_state.limit - 1) / flagged_state.limit;

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("Showing {} items", total))
                .size(12.0)
                .weak(),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Next page
            if ui
                .add_enabled(
                    flagged_state.offset + flagged_state.limit < total,
                    egui::Button::new("Next"),
                )
                .clicked()
            {
                flagged_state.offset += flagged_state.limit;
            }

            // Page indicator
            ui.label(format!("Page {} of {}", current_page, total_pages.max(1)));

            // Previous page
            if ui
                .add_enabled(flagged_state.offset > 0, egui::Button::new("Previous"))
                .clicked()
            {
                flagged_state.offset = (flagged_state.offset - flagged_state.limit).max(0);
            }
        });
    });
}

/// Renders delete confirmation dialog.
fn render_delete_dialog(
    ui: &mut egui::Ui,
    state: &mut AppState,
    flagged_state: &mut FlaggedState,
    id: i64,
) {
    egui::Window::new("Delete Flagged Item")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            ui.label("Are you sure you want to delete this flagged item?");
            ui.label(RichText::new("This action cannot be undone.").weak());

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    if let Err(e) = state.delete_flagged(id) {
                        state.set_error(e.to_string());
                    } else {
                        state.set_success("Item deleted");
                    }
                    flagged_state.show_delete_confirm = None;
                }

                if ui.button("Cancel").clicked() {
                    flagged_state.show_delete_confirm = None;
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flagged_state_default() {
        let state = FlaggedState::new();
        assert_eq!(state.offset, 0);
        assert_eq!(state.limit, 20);
        assert!(state.type_filter.is_none());
        assert!(!state.show_acknowledged);
    }

    #[test]
    fn test_flag_type_display() {
        assert_eq!(flag_type_display("distress").0, "Distress");
        assert_eq!(flag_type_display("crisis_indicator").0, "Crisis");
        assert_eq!(flag_type_display("bullying").0, "Bullying");
        assert_eq!(flag_type_display("negative_sentiment").0, "Negative");
        assert_eq!(flag_type_display("unknown").0, "Unknown");
    }
}

//! Main application struct and eframe integration.

use eframe::egui;
use egui::{Color32, RichText, Vec2};

use aegis_proxy::FilteringState;
use aegis_storage::Database;

use crate::state::{AppState, View};
use crate::theme::status;
use crate::views::{
    dashboard, flagged, login, logs, profiles, rules, settings, setup, system_logs,
};

/// Main dashboard application.
pub struct DashboardApp {
    /// Application state.
    state: AppState,

    /// Profile editor state.
    profile_editor: profiles::ProfileEditor,

    /// Rules view state.
    rules_state: rules::RulesState,

    /// Logs view state.
    logs_state: logs::LogsState,

    /// System logs view state.
    system_logs_state: system_logs::SystemLogsState,

    /// Settings view state.
    settings_state: settings::SettingsState,

    /// Flagged items view state.
    flagged_state: flagged::FlaggedState,

    /// Setup wizard state.
    setup_wizard: setup::SetupWizardState,
}

impl DashboardApp {
    /// Creates a new dashboard application.
    pub fn new(db: Database) -> Self {
        Self::with_filtering_state(db, None)
    }

    /// Creates a new dashboard application with an optional filtering state.
    ///
    /// If `filtering_state` is provided, rule changes made in the UI will be
    /// immediately applied to the running proxy.
    pub fn with_filtering_state(db: Database, filtering_state: Option<FilteringState>) -> Self {
        Self {
            state: AppState::with_filtering_state(db, filtering_state),
            profile_editor: profiles::ProfileEditor::default(),
            rules_state: rules::RulesState::new(),
            logs_state: logs::LogsState::new(),
            system_logs_state: system_logs::SystemLogsState::default(),
            settings_state: settings::SettingsState::default(),
            flagged_state: flagged::FlaggedState::new(),
            setup_wizard: setup::SetupWizardState::new(),
        }
    }

    /// Creates a new dashboard with in-memory database (for testing).
    pub fn in_memory() -> Result<Self, crate::error::UiError> {
        let db = Database::in_memory()?;
        Ok(Self::new(db))
    }

    /// Returns the window options for eframe.
    pub fn window_options() -> eframe::NativeOptions {
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([900.0, 600.0])
                .with_min_inner_size([700.0, 500.0])
                .with_title("Aegis Dashboard")
                .with_icon(Self::load_icon()),
            ..Default::default()
        }
    }

    /// Loads the application icon.
    fn load_icon() -> egui::IconData {
        let icon_data = include_bytes!("../../aegis-app/assets/icons/icon-256.png");
        let image = image::load_from_memory(icon_data)
            .expect("Failed to load icon")
            .into_rgba8();
        let (width, height) = image.dimensions();
        egui::IconData {
            rgba: image.into_raw(),
            width,
            height,
        }
    }

    /// Renders the sidebar navigation.
    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.set_min_width(180.0);

            // App header
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(RichText::new("Aegis").size(20.0).strong());
            });
            ui.add_space(8.0);

            // Status indicator
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                let status_color = self.state.protection_status.color();
                ui.colored_label(status_color, "●");
                ui.label(self.state.protection_status.as_str());
            });

            ui.add_space(24.0);
            ui.separator();
            ui.add_space(8.0);

            // Navigation items
            self.render_nav_item(ui, "Dashboard", View::Dashboard);
            self.render_nav_item(ui, "Profiles", View::Profiles);
            self.render_nav_item(ui, "Activity", View::Logs);
            self.render_nav_item_with_badge(ui, "Flagged", View::Flagged);
            self.render_nav_item(ui, "System Logs", View::SystemLogs);
            self.render_nav_item(ui, "Settings", View::Settings);

            // Spacer
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(16.0);

                // Lock button
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    if ui.button("Lock").clicked() {
                        self.state.lock();
                    }
                });

                ui.add_space(8.0);

                // Mode indicator
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new(format!("Mode: {}", self.state.interception_mode.as_str()))
                            .size(11.0)
                            .weak(),
                    );
                });

                // Version
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .size(10.0)
                            .weak(),
                    );
                });
            });
        });
    }

    /// Renders a navigation item.
    fn render_nav_item(&mut self, ui: &mut egui::Ui, label: &str, view: View) {
        let is_selected = self.state.view == view;

        let response =
            ui.allocate_response(Vec2::new(ui.available_width(), 36.0), egui::Sense::click());

        // Highlight background if selected or hovered
        let bg_color = if is_selected {
            ui.style().visuals.selection.bg_fill
        } else if response.hovered() {
            ui.style().visuals.widgets.hovered.bg_fill
        } else {
            Color32::TRANSPARENT
        };

        ui.painter().rect_filled(response.rect, 4.0, bg_color);

        // Draw text
        let text_color = if is_selected {
            ui.style().visuals.selection.stroke.color
        } else {
            ui.style().visuals.text_color()
        };

        ui.painter().text(
            response.rect.left_center() + Vec2::new(24.0, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::default(),
            text_color,
        );

        if response.clicked() && self.state.view != view {
            self.state.view = view;
            // Refresh data when switching views to update stats
            if let Err(e) = self.state.refresh_data() {
                tracing::warn!("Failed to refresh data on view switch: {}", e);
            }
        }
    }

    /// Renders a navigation item with an optional badge for unacknowledged count.
    fn render_nav_item_with_badge(&mut self, ui: &mut egui::Ui, label: &str, view: View) {
        let is_selected = self.state.view == view;
        let badge_count = self.state.unacknowledged_flagged_count();

        let response =
            ui.allocate_response(Vec2::new(ui.available_width(), 36.0), egui::Sense::click());

        // Highlight background if selected or hovered
        let bg_color = if is_selected {
            ui.style().visuals.selection.bg_fill
        } else if response.hovered() {
            ui.style().visuals.widgets.hovered.bg_fill
        } else {
            Color32::TRANSPARENT
        };

        ui.painter().rect_filled(response.rect, 4.0, bg_color);

        // Draw text
        let text_color = if is_selected {
            ui.style().visuals.selection.stroke.color
        } else {
            ui.style().visuals.text_color()
        };

        ui.painter().text(
            response.rect.left_center() + Vec2::new(24.0, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::default(),
            text_color,
        );

        // Draw badge if there are unacknowledged items
        if badge_count > 0 {
            let badge_text = if badge_count > 99 {
                "99+".to_string()
            } else {
                badge_count.to_string()
            };
            let badge_pos = response.rect.right_center() + Vec2::new(-24.0, 0.0);
            let badge_color = status::WARNING;

            // Badge background
            ui.painter().circle_filled(badge_pos, 10.0, badge_color);

            // Badge text
            ui.painter().text(
                badge_pos,
                egui::Align2::CENTER_CENTER,
                badge_text,
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );
        }

        if response.clicked() && self.state.view != view {
            self.state.view = view;
            // Refresh data when switching views to update stats
            if let Err(e) = self.state.refresh_data() {
                tracing::warn!("Failed to refresh data on view switch: {}", e);
            }
        }
    }

    /// Renders the header bar.
    #[allow(dead_code)]
    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // View title
            let title = match self.state.view {
                View::Login => "Login",
                View::Setup => "Setup",
                View::Dashboard => "Dashboard",
                View::Profiles => "Profiles",
                View::Rules => "Rules",
                View::Logs => "Activity Logs",
                View::Flagged => "Flagged Items",
                View::SystemLogs => "System Logs",
                View::Settings => "Settings",
            };
            ui.heading(title);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Lock button
                if ui.button("🔒 Lock").clicked() {
                    self.state.lock();
                }
            });
        });
    }

    /// Renders the main content area.
    fn render_content(&mut self, ui: &mut egui::Ui) {
        match self.state.view {
            View::Login | View::Setup => {
                // Login and Setup don't use header/sidebar
            }
            View::Dashboard => {
                dashboard::render(ui, &mut self.state);
            }
            View::Profiles => {
                profiles::render(ui, &mut self.state, &mut self.profile_editor);
            }
            View::Rules => {
                rules::render(ui, &mut self.state, &mut self.rules_state);
            }
            View::Logs => {
                logs::render(ui, &mut self.state, &mut self.logs_state);
            }
            View::Flagged => {
                flagged::render(ui, &mut self.state, &mut self.flagged_state);
            }
            View::SystemLogs => {
                system_logs::render(ui, &mut self.system_logs_state);
            }
            View::Settings => {
                settings::render(ui, &mut self.state, &mut self.settings_state);
            }
        }
    }

    /// Renders messages (error/success toasts).
    fn render_messages(&mut self, ctx: &egui::Context) {
        // Error message
        let mut clear_error = false;
        if let Some(error) = self.state.error_message.clone() {
            egui::TopBottomPanel::bottom("error_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(status::ERROR, "⚠");
                    ui.label(&error);
                    if ui.button("✕").clicked() {
                        clear_error = true;
                    }
                });
            });
        }
        if clear_error {
            self.state.error_message = None;
        }

        // Success message
        let mut clear_success = false;
        if let Some(success) = self.state.success_message.clone() {
            egui::TopBottomPanel::bottom("success_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(status::SUCCESS, "✓");
                    ui.label(&success);
                    if ui.button("✕").clicked() {
                        clear_success = true;
                    }
                });
            });
        }
        if clear_success {
            self.state.success_message = None;
        }
    }
}

impl eframe::App for DashboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update activity timestamp
        if self.state.is_authenticated() {
            self.state.touch_activity();
        }

        // Check for session expiry (but not during setup)
        if !self.state.is_authenticated()
            && self.state.view != View::Login
            && self.state.view != View::Setup
        {
            self.state.view = View::Login;
        }

        // Render based on view
        match self.state.view {
            View::Setup => {
                // Full-screen setup wizard
                egui::CentralPanel::default().show(ctx, |ui| {
                    setup::render(ui, &mut self.state, &mut self.setup_wizard);
                });
            }
            View::Login => {
                // Full-screen login
                egui::CentralPanel::default().show(ctx, |ui| {
                    login::render(ui, &mut self.state);
                });
            }
            _ => {
                // Sidebar + content layout
                egui::SidePanel::left("sidebar")
                    .resizable(false)
                    .default_width(180.0)
                    .show(ctx, |ui| {
                        self.render_sidebar(ui);
                    });

                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_content(ui);
                });
            }
        }

        // Messages overlay
        self.render_messages(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_app_creation() {
        let app = DashboardApp::in_memory().unwrap();
        // First setup starts with Setup view
        assert_eq!(app.state.view, View::Setup);
    }

    #[test]
    fn test_window_options() {
        let options = DashboardApp::window_options();
        assert!(options.viewport.inner_size.is_some());
    }
}

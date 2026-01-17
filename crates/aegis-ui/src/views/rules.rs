//! Rules configuration view with Community, Content, and Time rules.

use std::collections::HashMap;

use eframe::egui::{self, Color32, RichText};
use serde::{Deserialize, Serialize};

use aegis_core::classifier::Category;
use aegis_core::community_rules::{CommunityRuleManager, ParentOverrides, RuleTier};
use aegis_core::content_rules::{ContentAction, ContentRule, ContentRuleSet};
use aegis_core::time_rules::{TimeRange, TimeRuleSet, Weekday};

use crate::state::{AppState, RulesTab, View};
use crate::theme::{brand, status};

/// State for the rules view.
pub struct RulesState {
    /// Community rule manager.
    pub community_manager: CommunityRuleManager,
    /// Whether rules have been initialized from profile.
    pub initialized: bool,
    /// Content rules state per category.
    pub content_rules: HashMap<Category, ContentRuleState>,
    /// Time rules list.
    pub time_rules: Vec<TimeRule>,
    /// Parent overrides.
    pub overrides: ParentOverrides,
    /// Time rule editor.
    pub time_editor: TimeRuleEditor,
    /// Whitelist input field.
    pub whitelist_input: String,
    /// Blacklist input field.
    pub blacklist_input: String,
    /// Blacklist category selection.
    pub blacklist_category: Category,
    /// Whether there are unsaved changes.
    pub has_changes: bool,
    /// Currently selected community tier filter.
    pub community_tier_filter: Option<RuleTier>,
    /// Search filter for community rules.
    pub community_search: String,
}

impl Default for RulesState {
    fn default() -> Self {
        Self {
            community_manager: CommunityRuleManager::default(),
            initialized: false,
            content_rules: HashMap::new(),
            time_rules: Vec::new(),
            overrides: ParentOverrides::default(),
            time_editor: TimeRuleEditor::default(),
            whitelist_input: String::new(),
            blacklist_input: String::new(),
            blacklist_category: Category::Profanity, // Default to Profanity
            has_changes: false,
            community_tier_filter: None,
            community_search: String::new(),
        }
    }
}

/// State for a single content rule category.
#[derive(Clone)]
pub struct ContentRuleState {
    pub action: ContentAction,
    pub threshold: f32,
    pub enabled: bool,
}

impl Default for ContentRuleState {
    fn default() -> Self {
        Self {
            action: ContentAction::Block,
            threshold: 0.7,
            enabled: true,
        }
    }
}

/// A time rule configuration.
#[derive(Clone, Serialize, Deserialize)]
pub struct TimeRule {
    pub id: String,
    pub name: String,
    pub start_time: String,
    pub end_time: String,
    pub days: Vec<String>,
    pub action: String,
    pub enabled: bool,
}

impl Default for TimeRule {
    fn default() -> Self {
        Self {
            id: uuid_simple(),
            name: String::new(),
            start_time: "21:00".to_string(),
            end_time: "07:00".to_string(),
            days: vec![
                "Monday".to_string(),
                "Tuesday".to_string(),
                "Wednesday".to_string(),
                "Thursday".to_string(),
                "Friday".to_string(),
            ],
            action: "block".to_string(),
            enabled: true,
        }
    }
}

/// Editor state for time rules.
#[derive(Default)]
pub struct TimeRuleEditor {
    pub open: bool,
    pub editing_index: Option<usize>,
    pub rule: TimeRule,
    pub confirm_delete: Option<usize>,
}

impl TimeRuleEditor {
    pub fn new_rule(&mut self) {
        self.open = true;
        self.editing_index = None;
        self.rule = TimeRule::default();
    }

    pub fn edit_rule(&mut self, index: usize, rule: &TimeRule) {
        self.open = true;
        self.editing_index = Some(index);
        self.rule = rule.clone();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.editing_index = None;
        self.confirm_delete = None;
    }
}

/// Generate a simple unique ID.
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("rule_{:x}", nanos)
}

impl RulesState {
    /// Creates a new rules state.
    pub fn new() -> Self {
        Self {
            community_manager: CommunityRuleManager::with_defaults(),
            ..Default::default()
        }
    }

    /// Initializes state from a profile's rules.
    pub fn load_from_profile(&mut self, profile: &aegis_storage::Profile) {
        // Load content rules
        self.content_rules.clear();
        if let Some(rules) = profile
            .content_rules
            .get("categories")
            .and_then(|v| v.as_object())
        {
            for (cat_str, config) in rules {
                if let Some(category) = parse_category(cat_str) {
                    let state = ContentRuleState {
                        action: config
                            .get("action")
                            .and_then(|v| v.as_str())
                            .map(parse_action)
                            .unwrap_or(ContentAction::Block),
                        threshold: config
                            .get("threshold")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.7) as f32,
                        enabled: config
                            .get("enabled")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true),
                    };
                    self.content_rules.insert(category, state);
                }
            }
        }

        // Initialize defaults for missing categories
        for category in all_categories() {
            self.content_rules
                .entry(category)
                .or_insert_with(|| ContentRuleState {
                    action: ContentAction::Block,
                    threshold: default_threshold(category),
                    enabled: true,
                });
        }

        // Load time rules
        self.time_rules.clear();
        if let Some(rules) = profile.time_rules.get("rules").and_then(|v| v.as_array()) {
            for rule in rules {
                if let Ok(time_rule) = serde_json::from_value::<TimeRule>(rule.clone()) {
                    self.time_rules.push(time_rule);
                }
            }
        }

        // Load parent overrides
        if let Some(overrides) = profile.content_rules.get("overrides") {
            if let Ok(o) = serde_json::from_value::<ParentOverrides>(overrides.clone()) {
                self.overrides = o;
                self.community_manager.set_overrides(self.overrides.clone());
            }
        }

        self.initialized = true;
        self.has_changes = false;
    }

    /// Saves rules to a profile JSON format.
    pub fn save_to_profile(&self) -> (serde_json::Value, serde_json::Value) {
        // Build content rules JSON
        let mut categories = serde_json::Map::new();
        for (category, state) in &self.content_rules {
            let config = serde_json::json!({
                "action": action_to_str(state.action),
                "threshold": state.threshold,
                "enabled": state.enabled,
            });
            categories.insert(category_to_str(category), config);
        }

        let content_rules = serde_json::json!({
            "categories": categories,
            "overrides": self.overrides,
        });

        // Build time rules JSON
        let time_rules = serde_json::json!({
            "rules": self.time_rules,
        });

        (content_rules, time_rules)
    }
}

/// Renders the rules view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState, rules_state: &mut RulesState) {
    // Get selected profile name and initialize if needed
    let profile_name = state
        .selected_profile_id
        .and_then(|id| state.profiles.iter().find(|p| p.id == id))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    // Initialize rules state from profile if needed
    if !rules_state.initialized {
        if let Some(profile) = state
            .selected_profile_id
            .and_then(|id| state.profiles.iter().find(|p| p.id == id))
        {
            rules_state.load_from_profile(profile);
        }
    }

    // Track actions to perform after UI rendering
    let mut go_back = false;
    let mut save_changes = false;

    // Header with back button and save
    ui.horizontal(|ui| {
        if ui.button("< Back").clicked() {
            go_back = true;
        }
        ui.heading(format!("Rules: {}", profile_name));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let save_btn =
                ui.add_enabled(rules_state.has_changes, egui::Button::new("Save Changes"));
            if save_btn.clicked() {
                save_changes = true;
            }

            if rules_state.has_changes {
                ui.colored_label(status::WARNING, "Unsaved changes");
            }
        });
    });

    // Handle actions after UI
    if go_back {
        rules_state.initialized = false;
        state.view = View::Profiles;
        return;
    }
    if save_changes {
        save_rules(state, rules_state);
    }

    ui.add_space(16.0);

    // Tab bar
    ui.horizontal(|ui| {
        let tabs = [
            (RulesTab::Time, "Time Rules"),
            (RulesTab::Content, "Content Rules"),
            (RulesTab::Community, "Community Rules"),
        ];

        for (tab, label) in tabs {
            let selected = state.rules_tab == tab;
            if ui.selectable_label(selected, label).clicked() {
                state.rules_tab = tab;
            }
            ui.add_space(8.0);
        }
    });

    ui.separator();
    ui.add_space(8.0);

    // Content based on selected tab
    egui::ScrollArea::vertical().show(ui, |ui| match state.rules_tab {
        RulesTab::Time => render_time_rules(ui, state, rules_state),
        RulesTab::Content => render_content_rules(ui, rules_state),
        RulesTab::Community => render_community_rules(ui, rules_state),
    });

    // Time rule editor dialog
    if rules_state.time_editor.open {
        render_time_rule_editor(ui, rules_state);
    }
}

/// Renders the time rules tab.
fn render_time_rules(ui: &mut egui::Ui, state: &mut AppState, rules_state: &mut RulesState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Time-based Access Rules").strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("+ Add Rule").clicked() {
                rules_state.time_editor.new_rule();
            }
        });
    });

    ui.add_space(8.0);

    // Presets
    ui.label(RichText::new("Quick Presets").size(12.0).weak());
    ui.horizontal(|ui| {
        if ui.button("School Night (9pm-7am)").clicked() {
            let rule = TimeRule {
                id: uuid_simple(),
                name: "School Night Bedtime".to_string(),
                start_time: "21:00".to_string(),
                end_time: "07:00".to_string(),
                days: vec![
                    "Sunday".to_string(),
                    "Monday".to_string(),
                    "Tuesday".to_string(),
                    "Wednesday".to_string(),
                    "Thursday".to_string(),
                ],
                action: "block".to_string(),
                enabled: true,
            };
            rules_state.time_rules.push(rule);
            rules_state.has_changes = true;
            state.set_success("Added school night preset");
        }
        if ui.button("Weekend (11pm-8am)").clicked() {
            let rule = TimeRule {
                id: uuid_simple(),
                name: "Weekend Bedtime".to_string(),
                start_time: "23:00".to_string(),
                end_time: "08:00".to_string(),
                days: vec!["Friday".to_string(), "Saturday".to_string()],
                action: "block".to_string(),
                enabled: true,
            };
            rules_state.time_rules.push(rule);
            rules_state.has_changes = true;
            state.set_success("Added weekend preset");
        }
    });

    ui.add_space(16.0);

    // Rules list
    if rules_state.time_rules.is_empty() {
        render_empty_time_rules(ui);
    } else {
        // Clone rules for iteration to avoid borrow issues
        let rules_snapshot: Vec<(usize, TimeRule)> =
            rules_state.time_rules.iter().cloned().enumerate().collect();

        let mut delete_index = None;
        let mut edit_action: Option<(usize, TimeRule)> = None;

        for (i, rule) in rules_snapshot {
            let confirm_delete = rules_state.time_editor.confirm_delete;
            let action = render_time_rule_card_simple(ui, i, &rule, confirm_delete);
            match action {
                TimeRuleAction::Edit => {
                    edit_action = Some((i, rule));
                }
                TimeRuleAction::Delete => {
                    delete_index = Some(i);
                }
                TimeRuleAction::ConfirmDelete => {
                    rules_state.time_editor.confirm_delete = Some(i);
                }
                TimeRuleAction::CancelDelete => {
                    rules_state.time_editor.confirm_delete = None;
                }
                TimeRuleAction::None => {}
            }
            ui.add_space(4.0);
        }

        // Handle edit
        if let Some((idx, rule)) = edit_action {
            rules_state.time_editor.edit_rule(idx, &rule);
        }

        // Handle delete
        if let Some(idx) = delete_index {
            rules_state.time_rules.remove(idx);
            rules_state.has_changes = true;
            rules_state.time_editor.confirm_delete = None;
        }
    }
}

enum TimeRuleAction {
    None,
    Edit,
    Delete,
    ConfirmDelete,
    CancelDelete,
}

/// Renders empty state for time rules.
fn render_empty_time_rules(ui: &mut egui::Ui) {
    egui::Frame::new()
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .corner_radius(8.0)
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

/// Renders a single time rule card (simplified version without mutable rules_state).
fn render_time_rule_card_simple(
    ui: &mut egui::Ui,
    index: usize,
    rule: &TimeRule,
    confirm_delete: Option<usize>,
) -> TimeRuleAction {
    let mut action = TimeRuleAction::None;

    egui::Frame::new()
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Enabled toggle
                let status_color = if rule.enabled {
                    status::SUCCESS
                } else {
                    Color32::GRAY
                };
                ui.colored_label(status_color, "●");

                // Rule details
                ui.vertical(|ui| {
                    ui.label(&rule.name);
                    ui.label(
                        RichText::new(format!(
                            "{} - {} | {}",
                            rule.start_time,
                            rule.end_time,
                            rule.days.join(", ")
                        ))
                        .size(11.0)
                        .weak(),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Delete confirmation
                    if confirm_delete == Some(index) {
                        ui.colored_label(status::ERROR, "Delete?");
                        if ui.button("Yes").clicked() {
                            action = TimeRuleAction::Delete;
                        }
                        if ui.button("No").clicked() {
                            action = TimeRuleAction::CancelDelete;
                        }
                    } else {
                        if ui.button("Delete").clicked() {
                            action = TimeRuleAction::ConfirmDelete;
                        }
                        if ui.button("Edit").clicked() {
                            action = TimeRuleAction::Edit;
                        }
                    }
                });
            });
        });

    action
}

/// Renders the time rule editor dialog.
fn render_time_rule_editor(ui: &mut egui::Ui, rules_state: &mut RulesState) {
    let title = if rules_state.time_editor.editing_index.is_some() {
        "Edit Time Rule"
    } else {
        "New Time Rule"
    };

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            ui.set_min_width(400.0);

            // Name
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut rules_state.time_editor.rule.name);
            });

            ui.add_space(8.0);

            // Time range
            ui.horizontal(|ui| {
                ui.label("Start:");
                ui.add(
                    egui::TextEdit::singleline(&mut rules_state.time_editor.rule.start_time)
                        .desired_width(60.0),
                );
                ui.label("End:");
                ui.add(
                    egui::TextEdit::singleline(&mut rules_state.time_editor.rule.end_time)
                        .desired_width(60.0),
                );
            });
            ui.label(
                RichText::new("Use 24-hour format (HH:MM)")
                    .size(11.0)
                    .weak(),
            );

            ui.add_space(8.0);

            // Days
            ui.label("Days:");
            ui.horizontal_wrapped(|ui| {
                let all_days = [
                    "Monday",
                    "Tuesday",
                    "Wednesday",
                    "Thursday",
                    "Friday",
                    "Saturday",
                    "Sunday",
                ];
                for day in all_days {
                    let mut checked = rules_state.time_editor.rule.days.contains(&day.to_string());
                    if ui.checkbox(&mut checked, day).changed() {
                        if checked {
                            rules_state.time_editor.rule.days.push(day.to_string());
                        } else {
                            rules_state.time_editor.rule.days.retain(|d| d != day);
                        }
                    }
                }
            });

            ui.add_space(8.0);

            // Action
            ui.horizontal(|ui| {
                ui.label("Action:");
                egui::ComboBox::from_id_salt("time_action")
                    .selected_text(&rules_state.time_editor.rule.action)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut rules_state.time_editor.rule.action,
                            "block".to_string(),
                            "Block",
                        );
                        ui.selectable_value(
                            &mut rules_state.time_editor.rule.action,
                            "warn".to_string(),
                            "Warn",
                        );
                    });
            });

            // Enabled
            ui.checkbox(&mut rules_state.time_editor.rule.enabled, "Enabled");

            ui.add_space(16.0);

            // Buttons
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Save").clicked()
                        && !rules_state.time_editor.rule.name.trim().is_empty()
                    {
                        if let Some(idx) = rules_state.time_editor.editing_index {
                            rules_state.time_rules[idx] = rules_state.time_editor.rule.clone();
                        } else {
                            rules_state
                                .time_rules
                                .push(rules_state.time_editor.rule.clone());
                        }
                        rules_state.has_changes = true;
                        rules_state.time_editor.close();
                    }
                    if ui.button("Cancel").clicked() {
                        rules_state.time_editor.close();
                    }
                });
            });
        });
}

/// Renders the content rules tab.
fn render_content_rules(ui: &mut egui::Ui, rules_state: &mut RulesState) {
    ui.label(RichText::new("Content Category Rules").strong());
    ui.label(
        RichText::new("Configure how each content category is handled")
            .size(12.0)
            .weak(),
    );
    ui.add_space(12.0);

    // Category rules
    let categories = [
        (
            Category::Violence,
            "Violence",
            "Violent content and threats",
            status::ERROR,
        ),
        (
            Category::SelfHarm,
            "Self-Harm",
            "Self-harm and suicide content",
            status::ERROR,
        ),
        (
            Category::Adult,
            "Adult Content",
            "Sexual and adult material",
            status::WARNING,
        ),
        (
            Category::Jailbreak,
            "Jailbreak",
            "AI manipulation attempts",
            status::WARNING,
        ),
        (
            Category::Hate,
            "Hate Speech",
            "Discriminatory content",
            status::ERROR,
        ),
        (
            Category::Illegal,
            "Illegal",
            "Illegal activities",
            status::ERROR,
        ),
        (
            Category::Profanity,
            "Profanity",
            "Offensive language",
            Color32::GRAY,
        ),
    ];

    for (category, name, description, color) in categories {
        if render_content_rule_card(ui, category, name, description, color, rules_state) {
            rules_state.has_changes = true;
        }
        ui.add_space(6.0);
    }
}

/// Renders a single content rule card. Returns true if changed.
fn render_content_rule_card(
    ui: &mut egui::Ui,
    category: Category,
    name: &str,
    description: &str,
    color: Color32,
    rules_state: &mut RulesState,
) -> bool {
    let mut changed = false;

    let state = rules_state.content_rules.entry(category).or_default();

    egui::Frame::new()
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Category indicator and name
                ui.colored_label(color, "●");
                ui.vertical(|ui| {
                    ui.label(RichText::new(name).strong());
                    ui.label(RichText::new(description).size(11.0).weak());
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Enabled toggle
                    if ui.checkbox(&mut state.enabled, "").changed() {
                        changed = true;
                    }

                    // Action dropdown
                    let action_text = match state.action {
                        ContentAction::Block => "Block",
                        ContentAction::Warn => "Warn",
                        ContentAction::Allow => "Allow",
                    };

                    let mut action_idx = match state.action {
                        ContentAction::Block => 0,
                        ContentAction::Warn => 1,
                        ContentAction::Allow => 2,
                    };

                    egui::ComboBox::from_id_salt(format!("action_{:?}", category))
                        .selected_text(action_text)
                        .width(70.0)
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut action_idx, 0, "Block").changed() {
                                state.action = ContentAction::Block;
                                changed = true;
                            }
                            if ui.selectable_value(&mut action_idx, 1, "Warn").changed() {
                                state.action = ContentAction::Warn;
                                changed = true;
                            }
                            if ui.selectable_value(&mut action_idx, 2, "Allow").changed() {
                                state.action = ContentAction::Allow;
                                changed = true;
                            }
                        });

                    // Threshold slider
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("{:.0}%", state.threshold * 100.0)).size(11.0));
                    let slider = ui.add(
                        egui::Slider::new(&mut state.threshold, 0.0..=1.0)
                            .show_value(false)
                            .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                    );
                    if slider.changed() {
                        changed = true;
                    }
                    ui.label(RichText::new("Sensitivity:").size(11.0).weak());
                });
            });
        });

    changed
}

/// Renders the community rules tab.
fn render_community_rules(ui: &mut egui::Ui, rules_state: &mut RulesState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Community Rules").strong());
        ui.label(
            RichText::new(format!(
                "({} rules loaded)",
                rules_state.community_manager.rule_count()
            ))
            .size(12.0)
            .weak(),
        );
    });

    ui.add_space(12.0);

    // Tier explanation
    egui::Frame::new()
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new("Rule Priority (highest to lowest):")
                    .strong()
                    .size(12.0),
            );
            ui.horizontal(|ui| {
                ui.colored_label(brand::PRIMARY, "●");
                ui.label("Parent (your customizations)");
            });
            ui.horizontal(|ui| {
                ui.colored_label(brand::DARK, "●");
                ui.label("Curated (Aegis-maintained)");
            });
            ui.horizontal(|ui| {
                ui.colored_label(Color32::GRAY, "●");
                ui.label("Community (open-source databases)");
            });
        });

    ui.add_space(16.0);

    // Whitelist section
    ui.collapsing(RichText::new("Whitelist (Never Block)").strong(), |ui| {
        ui.label(
            RichText::new("Terms in this list will never be blocked, even if matched by rules")
                .size(11.0)
                .weak(),
        );
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut rules_state.whitelist_input);
            if ui.button("Add").clicked() && !rules_state.whitelist_input.trim().is_empty() {
                rules_state
                    .overrides
                    .add_whitelist(rules_state.whitelist_input.trim());
                rules_state
                    .community_manager
                    .set_overrides(rules_state.overrides.clone());
                rules_state.whitelist_input.clear();
                rules_state.has_changes = true;
            }
        });

        ui.add_space(8.0);

        // Display whitelist
        let whitelist: Vec<_> = rules_state.overrides.whitelist.iter().cloned().collect();
        if whitelist.is_empty() {
            ui.label(RichText::new("No whitelisted terms").weak().size(11.0));
        } else {
            ui.horizontal_wrapped(|ui| {
                let mut to_remove = None;
                for term in &whitelist {
                    ui.horizontal(|ui| {
                        egui::Frame::new()
                            .fill(Color32::from_rgb(0x22, 0xc5, 0x5e).gamma_multiply(0.2))
                            .corner_radius(4.0)
                            .inner_margin(4.0)
                            .show(ui, |ui| {
                                ui.label(term);
                                if ui.small_button("x").clicked() {
                                    to_remove = Some(term.clone());
                                }
                            });
                    });
                }
                if let Some(term) = to_remove {
                    rules_state.overrides.remove_whitelist(&term);
                    rules_state
                        .community_manager
                        .set_overrides(rules_state.overrides.clone());
                    rules_state.has_changes = true;
                }
            });
        }
    });

    ui.add_space(8.0);

    // Blacklist section
    ui.collapsing(RichText::new("Blacklist (Always Block)").strong(), |ui| {
        ui.label(
            RichText::new("Add custom terms to always block")
                .size(11.0)
                .weak(),
        );
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut rules_state.blacklist_input);

            egui::ComboBox::from_id_salt("blacklist_category")
                .selected_text(rules_state.blacklist_category.name())
                .width(100.0)
                .show_ui(ui, |ui| {
                    for cat in all_categories() {
                        ui.selectable_value(&mut rules_state.blacklist_category, cat, cat.name());
                    }
                });

            if ui.button("Add").clicked() && !rules_state.blacklist_input.trim().is_empty() {
                rules_state.overrides.add_blacklist(
                    rules_state.blacklist_input.trim(),
                    rules_state.blacklist_category,
                );
                rules_state
                    .community_manager
                    .set_overrides(rules_state.overrides.clone());
                rules_state.blacklist_input.clear();
                rules_state.has_changes = true;
            }
        });

        ui.add_space(8.0);

        // Display blacklist
        let blacklist: Vec<_> = rules_state
            .overrides
            .blacklist
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        if blacklist.is_empty() {
            ui.label(RichText::new("No blacklisted terms").weak().size(11.0));
        } else {
            ui.horizontal_wrapped(|ui| {
                let mut to_remove = None;
                for (term, category) in &blacklist {
                    ui.horizontal(|ui| {
                        egui::Frame::new()
                            .fill(Color32::from_rgb(0xef, 0x44, 0x44).gamma_multiply(0.2))
                            .corner_radius(4.0)
                            .inner_margin(4.0)
                            .show(ui, |ui| {
                                ui.label(format!("{} ({})", term, category.name()));
                                if ui.small_button("x").clicked() {
                                    to_remove = Some(term.clone());
                                }
                            });
                    });
                }
                if let Some(term) = to_remove {
                    rules_state.overrides.remove_blacklist(&term);
                    rules_state
                        .community_manager
                        .set_overrides(rules_state.overrides.clone());
                    rules_state.has_changes = true;
                }
            });
        }
    });

    ui.add_space(8.0);

    // Disabled rules section
    ui.collapsing(RichText::new("Disabled Rules").strong(), |ui| {
        ui.label(
            RichText::new("Rules you've disabled will not trigger")
                .size(11.0)
                .weak(),
        );
        ui.add_space(8.0);

        let disabled: Vec<_> = rules_state
            .overrides
            .disabled_rules
            .iter()
            .cloned()
            .collect();
        if disabled.is_empty() {
            ui.label(RichText::new("No disabled rules").weak().size(11.0));
        } else {
            let mut to_enable = None;
            for rule_id in &disabled {
                ui.horizontal(|ui| {
                    ui.label(rule_id);
                    if ui.small_button("Enable").clicked() {
                        to_enable = Some(rule_id.clone());
                    }
                });
            }
            if let Some(rule_id) = to_enable {
                rules_state.overrides.enable_rule(&rule_id);
                rules_state
                    .community_manager
                    .set_overrides(rules_state.overrides.clone());
                rules_state.has_changes = true;
            }
        }
    });

    ui.add_space(16.0);

    // Browse curated rules
    ui.collapsing(RichText::new("Browse Curated Rules").strong(), |ui| {
        ui.label(
            RichText::new("View and disable individual Aegis curated rules")
                .size(11.0)
                .weak(),
        );
        ui.add_space(8.0);

        // Search filter
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut rules_state.community_search);
        });

        ui.add_space(8.0);

        // Collect curated rules data first to avoid borrow issues
        let search_lower = rules_state.community_search.to_lowercase();
        let curated_rules: Vec<_> = rules_state
            .community_manager
            .rules_for_tier(RuleTier::Curated)
            .iter()
            .filter(|rule| {
                search_lower.is_empty() || rule.pattern.to_lowercase().contains(&search_lower)
            })
            .map(|rule| {
                (
                    rule.id.clone(),
                    rule.category,
                    rule.pattern.clone(),
                    rules_state.overrides.is_rule_disabled(&rule.id),
                )
            })
            .collect();

        let mut rule_to_toggle: Option<(String, bool)> = None;

        for (rule_id, category, pattern, is_disabled) in curated_rules {
            ui.horizontal(|ui| {
                // Category color
                let color = category_color(category);
                ui.colored_label(color, "●");

                // Rule info
                ui.vertical(|ui| {
                    ui.label(RichText::new(&rule_id).size(11.0));
                    ui.label(
                        RichText::new(format!(
                            "{} | {}",
                            category.name(),
                            truncate_pattern(&pattern, 40)
                        ))
                        .size(10.0)
                        .weak(),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if is_disabled {
                        if ui.button("Enable").clicked() {
                            rule_to_toggle = Some((rule_id.clone(), true));
                        }
                        ui.colored_label(Color32::GRAY, "Disabled");
                    } else {
                        if ui.button("Disable").clicked() {
                            rule_to_toggle = Some((rule_id.clone(), false));
                        }
                        ui.colored_label(status::SUCCESS, "Active");
                    }
                });
            });

            ui.add_space(4.0);
        }

        // Apply rule toggle after iteration
        if let Some((rule_id, enable)) = rule_to_toggle {
            if enable {
                rules_state.overrides.enable_rule(&rule_id);
            } else {
                rules_state.overrides.disable_rule(&rule_id);
            }
            rules_state
                .community_manager
                .set_overrides(rules_state.overrides.clone());
            rules_state.has_changes = true;
        }
    });
}

/// Saves rules to the profile.
fn save_rules(state: &mut AppState, rules_state: &mut RulesState) {
    if let Some(profile_id) = state.selected_profile_id {
        let (content_rules_json, time_rules_json) = rules_state.save_to_profile();

        // Find and update the profile
        if let Some(profile) = state.profiles.iter().find(|p| p.id == profile_id) {
            let updated = aegis_storage::NewProfile {
                name: profile.name.clone(),
                os_username: profile.os_username.clone(),
                time_rules: time_rules_json.clone(),
                content_rules: content_rules_json.clone(),
                enabled: profile.enabled,
                sentiment_config: profile.sentiment_config.clone(),
            };

            match state.db.update_profile(profile_id, updated) {
                Ok(()) => {
                    rules_state.has_changes = false;
                    state.set_success("Rules saved successfully");
                    let _ = state.refresh_data();

                    // Update the filtering state's rule engine if available
                    if let Some(ref filtering_state) = state.filtering_state {
                        // Convert the saved rules to core types
                        let time_rules = parse_time_rules_from_json(&time_rules_json);
                        let content_rules = parse_content_rules_from_json(&content_rules_json);

                        filtering_state.update_rules(time_rules, content_rules);
                        tracing::info!("Live rule update applied to proxy");
                    }
                }
                Err(e) => {
                    state.set_error(format!("Failed to save rules: {}", e));
                }
            }
        }
    }
}

/// Parses time rules from JSON into core TimeRuleSet.
fn parse_time_rules_from_json(json: &serde_json::Value) -> TimeRuleSet {
    let mut rule_set = TimeRuleSet::new();

    if let Some(rules) = json.get("rules").and_then(|v| v.as_array()) {
        for rule_json in rules {
            if let Ok(ui_rule) = serde_json::from_value::<TimeRule>(rule_json.clone()) {
                if !ui_rule.enabled {
                    continue; // Skip disabled rules
                }

                // Parse time range
                let start = parse_time(&ui_rule.start_time);
                let end = parse_time(&ui_rule.end_time);
                let time_range = TimeRange::new(start, end);

                // Parse days
                let days: Vec<Weekday> = ui_rule
                    .days
                    .iter()
                    .filter_map(|d| parse_weekday(d))
                    .collect();

                if !days.is_empty() {
                    let core_rule = aegis_core::time_rules::TimeRule::new(
                        &ui_rule.id,
                        &ui_rule.name,
                        days,
                        time_range,
                    );
                    rule_set.add_rule(core_rule);
                }
            }
        }
    }

    rule_set
}

/// Parses content rules from JSON into core ContentRuleSet.
fn parse_content_rules_from_json(json: &serde_json::Value) -> ContentRuleSet {
    let mut rule_set = ContentRuleSet::new();

    if let Some(categories) = json.get("categories").and_then(|v| v.as_object()) {
        for (cat_str, config) in categories {
            if let Some(category) = parse_category(cat_str) {
                let enabled = config
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                if !enabled {
                    continue; // Skip disabled rules
                }

                let action = config
                    .get("action")
                    .and_then(|v| v.as_str())
                    .map(parse_action)
                    .unwrap_or(ContentAction::Block);

                let threshold = config
                    .get("threshold")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.7) as f32;

                let rule = ContentRule::new(
                    format!("{:?}_rule", category).to_lowercase(),
                    format!("{:?} Rule", category),
                    category,
                    action,
                    threshold,
                );
                rule_set.add_rule(rule);
            }
        }
    }

    rule_set
}

/// Parses a time string (HH:MM) into TimeOfDay.
fn parse_time(s: &str) -> aegis_core::time_rules::TimeOfDay {
    let parts: Vec<&str> = s.split(':').collect();
    let hour = parts.first().and_then(|h| h.parse().ok()).unwrap_or(0);
    let minute = parts.get(1).and_then(|m| m.parse().ok()).unwrap_or(0);
    aegis_core::time_rules::TimeOfDay::new(hour, minute)
}

/// Parses a day name into Weekday.
fn parse_weekday(s: &str) -> Option<Weekday> {
    match s.to_lowercase().as_str() {
        "monday" => Some(Weekday::Monday),
        "tuesday" => Some(Weekday::Tuesday),
        "wednesday" => Some(Weekday::Wednesday),
        "thursday" => Some(Weekday::Thursday),
        "friday" => Some(Weekday::Friday),
        "saturday" => Some(Weekday::Saturday),
        "sunday" => Some(Weekday::Sunday),
        _ => None,
    }
}

// Helper functions

fn all_categories() -> Vec<Category> {
    vec![
        Category::Violence,
        Category::SelfHarm,
        Category::Adult,
        Category::Jailbreak,
        Category::Hate,
        Category::Illegal,
        Category::Profanity,
    ]
}

fn default_threshold(category: Category) -> f32 {
    match category {
        Category::SelfHarm => 0.5,
        Category::Jailbreak => 0.8,
        _ => 0.7,
    }
}

fn parse_category(s: &str) -> Option<Category> {
    match s.to_lowercase().as_str() {
        "violence" => Some(Category::Violence),
        "selfharm" | "self_harm" => Some(Category::SelfHarm),
        "adult" => Some(Category::Adult),
        "jailbreak" => Some(Category::Jailbreak),
        "hate" => Some(Category::Hate),
        "illegal" => Some(Category::Illegal),
        "profanity" => Some(Category::Profanity),
        _ => None,
    }
}

fn category_to_str(category: &Category) -> String {
    match category {
        Category::Violence => "violence",
        Category::SelfHarm => "selfharm",
        Category::Adult => "adult",
        Category::Jailbreak => "jailbreak",
        Category::Hate => "hate",
        Category::Illegal => "illegal",
        Category::Profanity => "profanity",
    }
    .to_string()
}

fn parse_action(s: &str) -> ContentAction {
    match s.to_lowercase().as_str() {
        "block" => ContentAction::Block,
        "warn" => ContentAction::Warn,
        "allow" => ContentAction::Allow,
        _ => ContentAction::Block,
    }
}

fn action_to_str(action: ContentAction) -> &'static str {
    match action {
        ContentAction::Block => "block",
        ContentAction::Warn => "warn",
        ContentAction::Allow => "allow",
    }
}

fn category_color(category: Category) -> Color32 {
    match category {
        Category::Violence | Category::SelfHarm | Category::Hate | Category::Illegal => {
            status::ERROR
        }
        Category::Adult | Category::Jailbreak => status::WARNING,
        Category::Profanity => Color32::GRAY,
    }
}

fn truncate_pattern(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rules_tab_default() {
        assert_eq!(RulesTab::default(), RulesTab::Time);
    }

    #[test]
    fn test_time_rule_default() {
        let rule = TimeRule::default();
        assert!(!rule.id.is_empty());
        assert_eq!(rule.start_time, "21:00");
        assert_eq!(rule.end_time, "07:00");
        assert!(rule.enabled);
    }

    #[test]
    fn test_content_rule_state_default() {
        let state = ContentRuleState::default();
        assert_eq!(state.action, ContentAction::Block);
        assert_eq!(state.threshold, 0.7);
        assert!(state.enabled);
    }

    #[test]
    fn test_rules_state_new() {
        let state = RulesState::new();
        assert!(!state.initialized);
        assert!(state.community_manager.rule_count() > 0);
    }
}

//! Rules configuration view with time and content rule editing.

use std::collections::HashSet;

use dioxus::prelude::*;

use aegis_core::classifier::Category;
use aegis_core::content_rules::{ContentAction, ContentRule, ContentRuleSet};
use aegis_core::time_rules::{TimeOfDay, TimeRange, TimeRule, TimeRuleSet, Weekday};

use crate::state::{AppState, RulesTab, View};

/// Rules view component.
#[component]
pub fn RulesView() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let selected_profile_id = state.read().selected_profile_id;
    let rules_tab = state.read().rules_tab;
    let profiles = state.read().profiles.clone();

    // Get profile name
    let profile_name = selected_profile_id
        .and_then(|id| profiles.iter().find(|p| p.id == id))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    rsx! {
        div {
            // Header
            div { class: "flex items-center gap-md mb-lg",
                button {
                    class: "btn btn-secondary btn-sm",
                    onclick: move |_| state.write().view = View::Profiles,
                    "< Back"
                }
                h1 { class: "text-lg font-bold", "Rules: {profile_name}" }
            }

            // Tabs
            div { class: "tabs",
                div {
                    class: if rules_tab == RulesTab::Time { "tab active" } else { "tab" },
                    onclick: move |_| state.write().rules_tab = RulesTab::Time,
                    "Time Rules"
                }
                div {
                    class: if rules_tab == RulesTab::Content { "tab active" } else { "tab" },
                    onclick: move |_| state.write().rules_tab = RulesTab::Content,
                    "Content Rules"
                }
                div {
                    class: if rules_tab == RulesTab::Community { "tab active" } else { "tab" },
                    onclick: move |_| state.write().rules_tab = RulesTab::Community,
                    "Community Rules"
                }
            }

            // Tab content
            div { class: "card mt-md",
                match rules_tab {
                    RulesTab::Time => rsx! { TimeRulesTab {} },
                    RulesTab::Content => rsx! { ContentRulesTab {} },
                    RulesTab::Community => rsx! { CommunityRulesTab {} },
                }
            }
        }
    }
}

/// Time rules tab with full CRUD functionality.
#[component]
fn TimeRulesTab() -> Element {
    let state = use_context::<Signal<AppState>>();
    let selected_profile_id = state.read().selected_profile_id;

    // Load time rules from profile
    let mut time_rules = use_signal(TimeRuleSet::new);
    let mut show_editor = use_signal(|| false);
    let mut editing_rule_id = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<String>);

    // Load rules on mount and when profile changes
    use_effect(move || {
        if let Some(profile_id) = selected_profile_id {
            if let Ok(Some(profile)) = state.read().db.get_profile(profile_id) {
                let rule_set = parse_time_rules(&profile.time_rules);
                time_rules.set(rule_set);
            }
        }
    });

    let rules = time_rules.read().rules.clone();

    rsx! {
        div {
            // Header
            div { class: "flex justify-between items-center mb-md",
                div {
                    h3 { class: "font-bold", "Time-based Access Rules" }
                    p { class: "text-sm text-muted", "Block AI access during specific hours (e.g., bedtime, school)." }
                }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| {
                        editing_rule_id.set(None);
                        show_editor.set(true);
                    },
                    "+ Add Rule"
                }
            }

            // Presets section
            div { class: "mb-lg",
                p { class: "text-sm text-muted mb-sm", "Quick Add Presets:" }
                div { class: "flex gap-sm flex-wrap",
                    button {
                        class: "btn btn-secondary btn-sm",
                        onclick: {
                            let mut time_rules = time_rules.clone();
                            let state = state.clone();
                            move |_| {
                                let preset = TimeRuleSet::bedtime_school_nights();
                                add_rule_if_not_exists(&mut time_rules, preset);
                                save_time_rules(&state, &time_rules);
                            }
                        },
                        "Bedtime (School Nights)"
                    }
                    button {
                        class: "btn btn-secondary btn-sm",
                        onclick: {
                            let mut time_rules = time_rules.clone();
                            let state = state.clone();
                            move |_| {
                                let preset = TimeRuleSet::bedtime_weekends();
                                add_rule_if_not_exists(&mut time_rules, preset);
                                save_time_rules(&state, &time_rules);
                            }
                        },
                        "Bedtime (Weekends)"
                    }
                    button {
                        class: "btn btn-secondary btn-sm",
                        onclick: {
                            let mut time_rules = time_rules.clone();
                            let state = state.clone();
                            move |_| {
                                let preset = TimeRuleSet::school_hours();
                                add_rule_if_not_exists(&mut time_rules, preset);
                                save_time_rules(&state, &time_rules);
                            }
                        },
                        "School Hours"
                    }
                }
            }

            // Rules list
            if rules.is_empty() {
                div { class: "empty-state",
                    p { class: "empty-state-text", "No time rules configured" }
                    p { class: "empty-state-subtext", "Add a rule or use a preset to restrict AI access during specific hours." }
                }
            } else {
                div { class: "space-y-sm",
                    for rule in rules.iter() {
                        {
                            let rule_id = rule.id.clone();
                            let rule_name = rule.name.clone();
                            let rule_enabled = rule.enabled;
                            let days_str = format_days(&rule.days);
                            let time_str = format_time_range(&rule.time_range);
                            let is_confirming = confirm_delete() == Some(rule_id.clone());

                            rsx! {
                                div { class: "rule-card",
                                    // Enable toggle - use a button styled as toggle for reliable clicks
                                    button {
                                        class: if rule_enabled { "btn btn-primary btn-sm" } else { "btn btn-secondary btn-sm" },
                                        style: "margin-right: 12px; min-width: 70px;",
                                        onclick: {
                                            let rule_id = rule_id.clone();
                                            let mut time_rules = time_rules.clone();
                                            let state = state.clone();
                                            move |_| {
                                                // Read current state from signal and toggle
                                                let current = time_rules.read()
                                                    .get_rule(&rule_id)
                                                    .map(|r| r.enabled)
                                                    .unwrap_or(false);
                                                let new_enabled = !current;
                                                tracing::info!("Toggling rule {} from {} to {}", rule_id, current, new_enabled);

                                                // Directly mutate the signal
                                                if let Some(rule) = time_rules.write().get_rule_mut(&rule_id) {
                                                    if new_enabled {
                                                        rule.enable();
                                                    } else {
                                                        rule.disable();
                                                    }
                                                }

                                                save_time_rules(&state, &time_rules);
                                            }
                                        },
                                        if rule_enabled { "Enabled" } else { "Disabled" }
                                    }

                                    // Rule info
                                    div { class: "rule-info", style: "flex: 1;",
                                        p { class: "rule-name",
                                            style: if !rule_enabled { "opacity: 0.5;" } else { "" },
                                            "{rule_name}"
                                        }
                                        p { class: "rule-description",
                                            style: if !rule_enabled { "opacity: 0.5;" } else { "" },
                                            "{days_str} | {time_str}"
                                        }
                                    }

                                    // Actions
                                    div { class: "rule-controls",
                                        if is_confirming {
                                            span { class: "text-sm", style: "color: var(--aegis-error); margin-right: 8px;", "Delete?" }
                                            button {
                                                class: "btn btn-danger btn-sm",
                                                onclick: {
                                                    let rule_id = rule_id.clone();
                                                    let mut time_rules = time_rules.clone();
                                                    let state = state.clone();
                                                    let mut confirm_delete = confirm_delete.clone();
                                                    move |_| {
                                                        delete_rule(&mut time_rules, &rule_id);
                                                        save_time_rules(&state, &time_rules);
                                                        confirm_delete.set(None);
                                                    }
                                                },
                                                "Yes"
                                            }
                                            button {
                                                class: "btn btn-secondary btn-sm",
                                                onclick: move |_| confirm_delete.set(None),
                                                "No"
                                            }
                                        } else {
                                            button {
                                                class: "btn btn-secondary btn-sm",
                                                onclick: {
                                                    let rule_id = rule_id.clone();
                                                    let mut editing_rule_id = editing_rule_id.clone();
                                                    let mut show_editor = show_editor.clone();
                                                    move |_| {
                                                        editing_rule_id.set(Some(rule_id.clone()));
                                                        show_editor.set(true);
                                                    }
                                                },
                                                "Edit"
                                            }
                                            button {
                                                class: "btn btn-secondary btn-sm",
                                                onclick: {
                                                    let rule_id = rule_id.clone();
                                                    let mut confirm_delete = confirm_delete.clone();
                                                    move |_| confirm_delete.set(Some(rule_id.clone()))
                                                },
                                                "Delete"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Editor modal - use key to force remount when editing different rule
        if show_editor() {
            TimeRuleEditorModal {
                key: "{editing_rule_id():?}",
                time_rules: time_rules,
                editing_rule_id: editing_rule_id(),
                on_close: move |_| show_editor.set(false),
                on_save: {
                    let state = state.clone();
                    let mut show_editor = show_editor.clone();
                    move |_| {
                        save_time_rules(&state, &time_rules);
                        show_editor.set(false);
                    }
                }
            }
        }
    }
}

/// Modal for creating/editing a time rule.
#[component]
fn TimeRuleEditorModal(
    time_rules: Signal<TimeRuleSet>,
    editing_rule_id: Option<String>,
    on_close: EventHandler<MouseEvent>,
    on_save: EventHandler<MouseEvent>,
) -> Element {
    let is_editing = editing_rule_id.is_some();
    let title = if is_editing { "Edit Time Rule" } else { "New Time Rule" };

    // Initialize form state from existing rule or defaults
    let existing_rule = editing_rule_id
        .as_ref()
        .and_then(|id| time_rules.read().get_rule(id).cloned());

    let mut rule_name = use_signal(|| {
        existing_rule
            .as_ref()
            .map(|r| r.name.clone())
            .unwrap_or_else(|| "New Rule".to_string())
    });

    let mut start_hour = use_signal(|| {
        existing_rule
            .as_ref()
            .map(|r| r.time_range.start.hour)
            .unwrap_or(21)
    });

    let mut start_minute = use_signal(|| {
        existing_rule
            .as_ref()
            .map(|r| r.time_range.start.minute)
            .unwrap_or(0)
    });

    let mut end_hour = use_signal(|| {
        existing_rule
            .as_ref()
            .map(|r| r.time_range.end.hour)
            .unwrap_or(7)
    });

    let mut end_minute = use_signal(|| {
        existing_rule
            .as_ref()
            .map(|r| r.time_range.end.minute)
            .unwrap_or(0)
    });

    let mut selected_days = use_signal(|| {
        existing_rule
            .as_ref()
            .map(|r| r.days.clone())
            .unwrap_or_else(|| Weekday::school_nights().into_iter().collect())
    });

    let mut error = use_signal(|| None::<String>);

    rsx! {
        div { class: "modal-overlay",
            div { class: "modal", style: "max-width: 500px;",
                div { class: "modal-header",
                    h3 { class: "modal-title", "{title}" }
                    button {
                        class: "modal-close",
                        onclick: move |evt| on_close.call(evt),
                        "x"
                    }
                }

                div { class: "modal-body",
                    // Rule name
                    div { class: "mb-md",
                        label { class: "text-sm font-bold", "Rule Name:" }
                        input {
                            class: "input",
                            value: "{rule_name}",
                            placeholder: "e.g., School Night Bedtime",
                            oninput: move |evt| rule_name.set(evt.value())
                        }
                    }

                    // Days selection
                    div { class: "mb-md",
                        label { class: "text-sm font-bold mb-sm", style: "display: block;", "Active Days:" }
                        div { class: "flex gap-sm flex-wrap",
                            for day in Weekday::all() {
                                {
                                    let is_selected = selected_days.read().contains(&day);
                                    let day_name = day_short_name(day);
                                    rsx! {
                                        button {
                                            class: if is_selected { "btn btn-primary btn-sm" } else { "btn btn-secondary btn-sm" },
                                            style: "min-width: 45px;",
                                            onclick: {
                                                let mut selected_days = selected_days.clone();
                                                move |_| {
                                                    let mut days = selected_days.read().clone();
                                                    if days.contains(&day) {
                                                        days.remove(&day);
                                                    } else {
                                                        days.insert(day);
                                                    }
                                                    selected_days.set(days);
                                                }
                                            },
                                            "{day_name}"
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "flex gap-sm mt-sm",
                            button {
                                class: "btn btn-secondary btn-sm",
                                onclick: move |_| selected_days.set(Weekday::weekdays().into_iter().collect()),
                                "Weekdays"
                            }
                            button {
                                class: "btn btn-secondary btn-sm",
                                onclick: move |_| selected_days.set(Weekday::weekends().into_iter().collect()),
                                "Weekends"
                            }
                            button {
                                class: "btn btn-secondary btn-sm",
                                onclick: move |_| selected_days.set(Weekday::all().into_iter().collect()),
                                "All"
                            }
                            button {
                                class: "btn btn-secondary btn-sm",
                                onclick: move |_| selected_days.set(Weekday::school_nights().into_iter().collect()),
                                "School Nights"
                            }
                        }
                    }

                    // Time range
                    div { class: "mb-md",
                        label { class: "text-sm font-bold mb-sm", style: "display: block;", "Blocked Time Range:" }
                        div { class: "flex items-center gap-sm",
                            div { class: "flex items-center gap-xs",
                                select {
                                    class: "select",
                                    style: "width: 70px;",
                                    onchange: move |evt| {
                                        if let Ok(h) = evt.value().parse::<u8>() {
                                            start_hour.set(h);
                                        }
                                    },
                                    for h in 0u8..24 {
                                        option {
                                            value: "{h}",
                                            selected: h == start_hour(),
                                            "{h:02}"
                                        }
                                    }
                                }
                                span { ":" }
                                select {
                                    class: "select",
                                    style: "width: 70px;",
                                    onchange: move |evt| {
                                        if let Ok(m) = evt.value().parse::<u8>() {
                                            start_minute.set(m);
                                        }
                                    },
                                    for m in [0u8, 15, 30, 45] {
                                        option {
                                            value: "{m}",
                                            selected: m == start_minute(),
                                            "{m:02}"
                                        }
                                    }
                                }
                            }

                            span { class: "text-muted", "to" }

                            div { class: "flex items-center gap-xs",
                                select {
                                    class: "select",
                                    style: "width: 70px;",
                                    onchange: move |evt| {
                                        if let Ok(h) = evt.value().parse::<u8>() {
                                            end_hour.set(h);
                                        }
                                    },
                                    for h in 0u8..24 {
                                        option {
                                            value: "{h}",
                                            selected: h == end_hour(),
                                            "{h:02}"
                                        }
                                    }
                                }
                                span { ":" }
                                select {
                                    class: "select",
                                    style: "width: 70px;",
                                    onchange: move |evt| {
                                        if let Ok(m) = evt.value().parse::<u8>() {
                                            end_minute.set(m);
                                        }
                                    },
                                    for m in [0u8, 15, 30, 45] {
                                        option {
                                            value: "{m}",
                                            selected: m == end_minute(),
                                            "{m:02}"
                                        }
                                    }
                                }
                            }
                        }

                        // Preview
                        {
                            let start = TimeOfDay::new(start_hour(), start_minute());
                            let end = TimeOfDay::new(end_hour(), end_minute());
                            let range = TimeRange::new(start, end);
                            let preview = if range.is_overnight() {
                                format!("Blocks from {}:{:02} until {}:{:02} the next day", start_hour(), start_minute(), end_hour(), end_minute())
                            } else {
                                format!("Blocks from {}:{:02} to {}:{:02}", start_hour(), start_minute(), end_hour(), end_minute())
                            };
                            rsx! {
                                p { class: "text-sm text-muted mt-sm", "{preview}" }
                            }
                        }
                    }

                    // Error display
                    if let Some(err) = error() {
                        div { class: "auth-error", "{err}" }
                    }
                }

                div { class: "modal-footer",
                    button {
                        class: "btn btn-secondary",
                        onclick: move |evt| on_close.call(evt),
                        "Cancel"
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: {
                            let editing_rule_id = editing_rule_id.clone();
                            let mut time_rules = time_rules.clone();
                            move |evt| {
                                let name = rule_name().trim().to_string();
                                if name.is_empty() {
                                    error.set(Some("Rule name is required".to_string()));
                                    return;
                                }
                                if selected_days.read().is_empty() {
                                    error.set(Some("Select at least one day".to_string()));
                                    return;
                                }

                                let start = TimeOfDay::new(start_hour(), start_minute());
                                let end = TimeOfDay::new(end_hour(), end_minute());
                                let range = TimeRange::new(start, end);

                                if let Some(ref existing_id) = editing_rule_id {
                                    // Update existing rule
                                    if let Some(rule) = time_rules.write().get_rule_mut(existing_id) {
                                        rule.name = name;
                                        rule.days = selected_days.read().clone();
                                        rule.time_range = range;
                                    }
                                } else {
                                    // Create new rule
                                    let id = generate_rule_id();
                                    let rule = TimeRule::new(
                                        id,
                                        name,
                                        selected_days.read().iter().copied().collect::<Vec<_>>(),
                                        range,
                                    );
                                    time_rules.write().add_rule(rule);
                                }

                                on_save.call(evt);
                            }
                        },
                        "Save"
                    }
                }
            }
        }
    }
}

/// Content rules tab with full CRUD functionality.
#[component]
fn ContentRulesTab() -> Element {
    let state = use_context::<Signal<AppState>>();
    let selected_profile_id = state.read().selected_profile_id;

    // Load content rules from profile
    let mut content_rules = use_signal(ContentRuleSet::new);

    // Load rules on mount and when profile changes
    use_effect(move || {
        if let Some(profile_id) = selected_profile_id {
            if let Ok(Some(profile)) = state.read().db.get_profile(profile_id) {
                let rule_set = parse_content_rules(&profile.content_rules);
                content_rules.set(rule_set);
            }
        }
    });

    rsx! {
        div {
            // Header
            div { class: "flex justify-between items-center mb-md",
                div {
                    h3 { class: "font-bold", "Content Category Rules" }
                    p { class: "text-sm text-muted", "Configure how each content category is handled based on detection confidence." }
                }
            }

            // Presets section
            div { class: "mb-lg",
                p { class: "text-sm text-muted mb-sm", "Quick Presets:" }
                div { class: "flex gap-sm flex-wrap",
                    button {
                        class: "btn btn-secondary btn-sm",
                        onclick: {
                            let mut content_rules = content_rules.clone();
                            let state = state.clone();
                            move |_| {
                                content_rules.set(create_family_safe_preset());
                                save_content_rules(&state, &content_rules);
                            }
                        },
                        "Family Safe (Recommended)"
                    }
                    button {
                        class: "btn btn-secondary btn-sm",
                        onclick: {
                            let mut content_rules = content_rules.clone();
                            let state = state.clone();
                            move |_| {
                                content_rules.set(create_permissive_preset());
                                save_content_rules(&state, &content_rules);
                            }
                        },
                        "Permissive (Warn Only)"
                    }
                }
            }

            // Category list
            div { class: "space-y-sm",
                ContentCategoryRow {
                    category: Category::Violence,
                    description: "Violent content and threats",
                    color: "var(--aegis-error)",
                    content_rules: content_rules,
                    state: state,
                }
                ContentCategoryRow {
                    category: Category::SelfHarm,
                    description: "Self-harm and suicide content",
                    color: "var(--aegis-error)",
                    content_rules: content_rules,
                    state: state,
                }
                ContentCategoryRow {
                    category: Category::Adult,
                    description: "Sexual and adult material",
                    color: "var(--aegis-warning)",
                    content_rules: content_rules,
                    state: state,
                }
                ContentCategoryRow {
                    category: Category::Jailbreak,
                    description: "AI manipulation attempts",
                    color: "var(--aegis-warning)",
                    content_rules: content_rules,
                    state: state,
                }
                ContentCategoryRow {
                    category: Category::Hate,
                    description: "Discriminatory content",
                    color: "var(--aegis-error)",
                    content_rules: content_rules,
                    state: state,
                }
                ContentCategoryRow {
                    category: Category::Illegal,
                    description: "Illegal activities",
                    color: "var(--aegis-error)",
                    content_rules: content_rules,
                    state: state,
                }
                ContentCategoryRow {
                    category: Category::Profanity,
                    description: "Offensive language",
                    color: "var(--aegis-slate-400)",
                    content_rules: content_rules,
                    state: state,
                }
            }

            // Legend
            div { class: "mt-lg",
                p { class: "text-sm font-bold mb-sm", "Sensitivity Guide:" }
                div { class: "text-sm text-muted",
                    p { "Low (0.9): Only block very obvious violations" }
                    p { "Medium (0.7): Balanced detection (recommended)" }
                    p { "High (0.5): More aggressive, may have false positives" }
                }
            }
        }
    }
}

/// Content category row with action and threshold controls.
#[component]
fn ContentCategoryRow(
    category: Category,
    description: &'static str,
    color: &'static str,
    content_rules: Signal<ContentRuleSet>,
    state: Signal<AppState>,
) -> Element {
    let category_name = category.name();
    let rule_id = get_rule_id_for_category(category);

    // Get current rule state
    let (current_action, current_threshold, is_enabled) = {
        let rules = content_rules.read();
        if let Some(rule) = rules.get_rule(&rule_id) {
            (rule.action, rule.threshold, rule.enabled)
        } else {
            // Default: Block at 0.7 threshold
            (ContentAction::Block, 0.7, true)
        }
    };

    rsx! {
        div { class: "rule-card",
            // Category indicator
            span { class: "rule-category-dot", style: "background-color: {color};" }

            // Enable/disable toggle
            button {
                class: if is_enabled { "btn btn-primary btn-sm" } else { "btn btn-secondary btn-sm" },
                style: "margin-right: 12px; min-width: 70px;",
                onclick: {
                    let rule_id = rule_id.clone();
                    let mut content_rules = content_rules.clone();
                    let state = state.clone();
                    move |_| {
                        ensure_rule_exists(&mut content_rules, category, &rule_id);
                        let new_enabled = !content_rules.read().get_rule(&rule_id).map(|r| r.enabled).unwrap_or(true);
                        content_rules.write().set_rule_enabled(&rule_id, new_enabled);
                        save_content_rules(&state, &content_rules);
                    }
                },
                if is_enabled { "Enabled" } else { "Disabled" }
            }

            // Category info
            div { class: "rule-info", style: "flex: 1; min-width: 150px;",
                p { class: "rule-name",
                    style: if !is_enabled { "opacity: 0.5;" } else { "" },
                    "{category_name}"
                }
                p { class: "rule-description",
                    style: if !is_enabled { "opacity: 0.5;" } else { "" },
                    "{description}"
                }
            }

            // Action selector and threshold - fixed width container
            div { class: "rule-controls", style: "display: flex; align-items: center; gap: 12px; min-width: 240px;",
                div { style: "display: flex; flex-direction: column; gap: 4px;",
                    label { class: "text-xs text-muted", "Action" }
                    select {
                        class: "select",
                        style: "min-width: 80px;",
                        disabled: !is_enabled,
                        onchange: {
                            let rule_id = rule_id.clone();
                            let mut content_rules = content_rules.clone();
                            let state = state.clone();
                            move |evt: Event<FormData>| {
                                let action = string_to_action(&evt.value());
                                ensure_rule_exists(&mut content_rules, category, &rule_id);
                                content_rules.write().set_rule_action(&rule_id, action);
                                save_content_rules(&state, &content_rules);
                            }
                        },
                        option { value: "block", selected: current_action == ContentAction::Block, "Block" }
                        option { value: "warn", selected: current_action == ContentAction::Warn, "Warn" }
                        option { value: "allow", selected: current_action == ContentAction::Allow, "Allow" }
                    }
                }

                // Threshold slider - always render container for stable layout
                div { style: "display: flex; flex-direction: column; gap: 4px; min-width: 130px;",
                    if current_action != ContentAction::Allow {
                        label { class: "text-xs text-muted", "Sensitivity: {format_threshold(current_threshold)}" }
                        input {
                            r#type: "range",
                            class: "slider",
                            style: "width: 100px;",
                            disabled: !is_enabled,
                            min: "30",
                            max: "95",
                            step: "5",
                            value: "{(current_threshold * 100.0) as i32}",
                            onchange: {
                                let rule_id = rule_id.clone();
                                let mut content_rules = content_rules.clone();
                                let state = state.clone();
                                move |evt: Event<FormData>| {
                                    if let Ok(val) = evt.value().parse::<f32>() {
                                        let threshold = val / 100.0;
                                        ensure_rule_exists(&mut content_rules, category, &rule_id);
                                        content_rules.write().set_rule_threshold(&rule_id, threshold);
                                        save_content_rules(&state, &content_rules);
                                    }
                                }
                            }
                        }
                    } else {
                        // Placeholder to maintain layout
                        label { class: "text-xs text-muted", style: "visibility: hidden;", "Sensitivity: N/A" }
                        div { style: "width: 100px; height: 20px;" }
                    }
                }
            }
        }
    }
}

/// Community rules tab.
#[component]
fn CommunityRulesTab() -> Element {
    rsx! {
        div {
            h3 { class: "font-bold mb-md", "Community Rules" }
            p { class: "text-muted mb-md", "Rules from the Aegis community database." }

            div { class: "card mb-md",
                p { class: "font-bold", "Rule Priority (highest to lowest):" }
                p { "1. Parent (your customizations)" }
                p { "2. Curated (Aegis-maintained)" }
                p { "3. Community (open-source databases)" }
            }

            // Whitelist/Blacklist sections
            div { class: "mb-md",
                h4 { class: "font-bold mb-sm", "Whitelist (Never Block)" }
                p { class: "text-sm text-muted", "Terms in this list will never be blocked." }
                div { class: "flex gap-sm mt-sm",
                    input { class: "input", style: "flex: 1;", placeholder: "Add term..." }
                    button { class: "btn btn-primary btn-sm", "Add" }
                }
            }

            div { class: "mb-md",
                h4 { class: "font-bold mb-sm", "Blacklist (Always Block)" }
                p { class: "text-sm text-muted", "Add custom terms to always block." }
                div { class: "flex gap-sm mt-sm",
                    input { class: "input", style: "flex: 1;", placeholder: "Add term..." }
                    button { class: "btn btn-primary btn-sm", "Add" }
                }
            }
        }
    }
}

// === Helper Functions ===

/// Parses time rules JSON into TimeRuleSet.
fn parse_time_rules(json: &serde_json::Value) -> TimeRuleSet {
    // Try to parse as TimeRuleSet directly
    if let Ok(rule_set) = serde_json::from_value::<TimeRuleSet>(json.clone()) {
        return rule_set;
    }

    // Try to parse old format: { "rules": [...] }
    if let Some(rules_array) = json.get("rules").and_then(|r| r.as_array()) {
        let mut rule_set = TimeRuleSet::new();
        for rule_json in rules_array {
            if let Ok(rule) = parse_legacy_rule(rule_json) {
                rule_set.add_rule(rule);
            }
        }
        return rule_set;
    }

    TimeRuleSet::new()
}

/// Parses a legacy rule format.
fn parse_legacy_rule(json: &serde_json::Value) -> Result<TimeRule, ()> {
    let name = json
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or(())?
        .to_string();
    let enabled = json.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);

    // Parse start time (e.g., "21:00")
    let start_str = json.get("start_time").and_then(|s| s.as_str()).ok_or(())?;
    let start = parse_time_str(start_str)?;

    // Parse end time
    let end_str = json.get("end_time").and_then(|e| e.as_str()).ok_or(())?;
    let end = parse_time_str(end_str)?;

    // Parse days
    let days_array = json.get("days").and_then(|d| d.as_array()).ok_or(())?;
    let days: Vec<Weekday> = days_array
        .iter()
        .filter_map(|d| d.as_str().and_then(parse_weekday))
        .collect();

    if days.is_empty() {
        return Err(());
    }

    let id = generate_rule_id();
    let mut rule = TimeRule::new(id, name, days, TimeRange::new(start, end));
    if !enabled {
        rule.disable();
    }

    Ok(rule)
}

/// Parses a time string like "21:00" into TimeOfDay.
fn parse_time_str(s: &str) -> Result<TimeOfDay, ()> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(());
    }
    let hour: u8 = parts[0].parse().map_err(|_| ())?;
    let minute: u8 = parts[1].parse().map_err(|_| ())?;
    if hour >= 24 || minute >= 60 {
        return Err(());
    }
    Ok(TimeOfDay::new(hour, minute))
}

/// Parses a weekday string.
fn parse_weekday(s: &str) -> Option<Weekday> {
    match s.to_lowercase().as_str() {
        "monday" | "mon" => Some(Weekday::Monday),
        "tuesday" | "tue" => Some(Weekday::Tuesday),
        "wednesday" | "wed" => Some(Weekday::Wednesday),
        "thursday" | "thu" => Some(Weekday::Thursday),
        "friday" | "fri" => Some(Weekday::Friday),
        "saturday" | "sat" => Some(Weekday::Saturday),
        "sunday" | "sun" => Some(Weekday::Sunday),
        _ => None,
    }
}

/// Generates a unique rule ID.
fn generate_rule_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("rule_{}", timestamp)
}

/// Formats days as a readable string.
fn format_days(days: &HashSet<Weekday>) -> String {
    let weekdays: HashSet<_> = Weekday::weekdays().into_iter().collect();
    let weekends: HashSet<_> = Weekday::weekends().into_iter().collect();
    let all: HashSet<_> = Weekday::all().into_iter().collect();
    let school_nights: HashSet<_> = Weekday::school_nights().into_iter().collect();

    if days == &all {
        return "Every day".to_string();
    }
    if days == &weekdays {
        return "Weekdays".to_string();
    }
    if days == &weekends {
        return "Weekends".to_string();
    }
    if days == &school_nights {
        return "School nights".to_string();
    }

    // List individual days
    let mut sorted: Vec<_> = days.iter().copied().collect();
    sorted.sort_by_key(|d| match d {
        Weekday::Monday => 0,
        Weekday::Tuesday => 1,
        Weekday::Wednesday => 2,
        Weekday::Thursday => 3,
        Weekday::Friday => 4,
        Weekday::Saturday => 5,
        Weekday::Sunday => 6,
    });

    sorted
        .iter()
        .map(|d| day_short_name(*d))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Returns short day name.
fn day_short_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Monday => "Mon",
        Weekday::Tuesday => "Tue",
        Weekday::Wednesday => "Wed",
        Weekday::Thursday => "Thu",
        Weekday::Friday => "Fri",
        Weekday::Saturday => "Sat",
        Weekday::Sunday => "Sun",
    }
}

/// Formats a time range as a readable string.
fn format_time_range(range: &TimeRange) -> String {
    let start = format!("{:02}:{:02}", range.start.hour, range.start.minute);
    let end = format!("{:02}:{:02}", range.end.hour, range.end.minute);

    if range.is_overnight() {
        format!("{} - {} (next day)", start, end)
    } else {
        format!("{} - {}", start, end)
    }
}

/// Adds a rule if one with the same ID doesn't exist.
fn add_rule_if_not_exists(time_rules: &mut Signal<TimeRuleSet>, rule: TimeRule) {
    let exists = time_rules.read().get_rule(&rule.id).is_some();
    if !exists {
        time_rules.write().add_rule(rule);
    }
}

/// Toggles a rule's enabled state.
fn toggle_rule_enabled(time_rules: &mut Signal<TimeRuleSet>, rule_id: &str, enabled: bool) {
    if let Some(rule) = time_rules.write().get_rule_mut(rule_id) {
        if enabled {
            rule.enable();
        } else {
            rule.disable();
        }
    }
}

/// Deletes a rule by ID.
fn delete_rule(time_rules: &mut Signal<TimeRuleSet>, rule_id: &str) {
    time_rules.write().remove_rule(rule_id);
}

/// Saves time rules to the database and notifies the proxy.
fn save_time_rules(state: &Signal<AppState>, time_rules: &Signal<TimeRuleSet>) {
    let state_ref = state.read();
    let Some(profile_id) = state_ref.selected_profile_id else {
        return;
    };

    let Ok(Some(profile)) = state_ref.db.get_profile(profile_id) else {
        return;
    };

    // Convert TimeRuleSet to JSON
    let time_rules_json = serde_json::to_value(time_rules.read().clone()).unwrap_or_default();

    let updated_profile = aegis_storage::NewProfile {
        name: profile.name,
        os_username: profile.os_username,
        time_rules: time_rules_json,
        content_rules: profile.content_rules,
        enabled: profile.enabled,
        sentiment_config: profile.sentiment_config,
    };

    if let Err(e) = state_ref.db.update_profile(profile_id, updated_profile) {
        tracing::error!("Failed to save time rules: {}", e);
        return;
    }

    tracing::info!("Time rules saved for profile {}", profile_id);

    // Notify the proxy to reload rules
    reload_rules_from_api(profile_id);
}

/// Calls the API to reload rules into the proxy.
fn reload_rules_from_api(profile_id: i64) {
    // Use a separate thread to avoid blocking the UI
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        let url = "http://127.0.0.1:48765/api/rules/reload";

        match client
            .post(url)
            .json(&serde_json::json!({ "profile_id": profile_id }))
            .send()
        {
            Ok(response) => {
                if response.status().is_success() {
                    tracing::info!("Rules reloaded in proxy for profile {}", profile_id);
                } else {
                    tracing::warn!(
                        "Failed to reload rules: HTTP {}",
                        response.status()
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Failed to call reload API: {}", e);
            }
        }
    });
}

// === Content Rules Helper Functions ===

/// Parses content rules JSON into ContentRuleSet.
fn parse_content_rules(json: &serde_json::Value) -> ContentRuleSet {
    // Try to parse as ContentRuleSet directly
    if let Ok(rule_set) = serde_json::from_value::<ContentRuleSet>(json.clone()) {
        // If successfully parsed but empty, return defaults
        if rule_set.rules.is_empty() {
            return create_full_defaults();
        }
        return rule_set;
    }

    // Return defaults if parsing fails
    create_full_defaults()
}

/// Creates a full default rule set with all 7 categories (same as family safe).
fn create_full_defaults() -> ContentRuleSet {
    create_family_safe_preset()
}

/// Creates Family Safe preset - blocks all categories with balanced thresholds.
fn create_family_safe_preset() -> ContentRuleSet {
    ContentRuleSet {
        rules: vec![
            ContentRule::new("violence_block", "Block Violence", Category::Violence, ContentAction::Block, 0.7),
            ContentRule::new("selfharm_block", "Block Self-Harm", Category::SelfHarm, ContentAction::Block, 0.5),
            ContentRule::new("adult_block", "Block Adult", Category::Adult, ContentAction::Block, 0.7),
            ContentRule::new("jailbreak_block", "Block Jailbreak", Category::Jailbreak, ContentAction::Block, 0.8),
            ContentRule::new("hate_block", "Block Hate", Category::Hate, ContentAction::Block, 0.7),
            ContentRule::new("illegal_block", "Block Illegal", Category::Illegal, ContentAction::Block, 0.7),
            ContentRule::new("profanity_block", "Block Profanity", Category::Profanity, ContentAction::Block, 0.8),
        ],
    }
}

/// Creates Permissive preset - warns instead of blocking (except self-harm).
fn create_permissive_preset() -> ContentRuleSet {
    ContentRuleSet {
        rules: vec![
            ContentRule::new("violence_block", "Warn Violence", Category::Violence, ContentAction::Warn, 0.8),
            ContentRule::new("selfharm_block", "Block Self-Harm", Category::SelfHarm, ContentAction::Block, 0.5), // Always block self-harm
            ContentRule::new("adult_block", "Warn Adult", Category::Adult, ContentAction::Warn, 0.8),
            ContentRule::new("jailbreak_block", "Warn Jailbreak", Category::Jailbreak, ContentAction::Warn, 0.9),
            ContentRule::new("hate_block", "Warn Hate", Category::Hate, ContentAction::Warn, 0.8),
            ContentRule::new("illegal_block", "Warn Illegal", Category::Illegal, ContentAction::Warn, 0.8),
            ContentRule::new("profanity_block", "Allow Profanity", Category::Profanity, ContentAction::Allow, 0.0),
        ],
    }
}

/// Gets the rule ID for a category.
fn get_rule_id_for_category(category: Category) -> String {
    match category {
        Category::Violence => "violence_block".to_string(),
        Category::SelfHarm => "selfharm_block".to_string(),
        Category::Adult => "adult_block".to_string(),
        Category::Jailbreak => "jailbreak_block".to_string(),
        Category::Hate => "hate_block".to_string(),
        Category::Illegal => "illegal_block".to_string(),
        Category::Profanity => "profanity_block".to_string(),
    }
}

/// Ensures a rule exists for the given category.
fn ensure_rule_exists(content_rules: &mut Signal<ContentRuleSet>, category: Category, rule_id: &str) {
    if content_rules.read().get_rule(rule_id).is_none() {
        // Add default rule for this category
        let rule = ContentRule::block(rule_id, category, 0.7);
        content_rules.write().add_rule(rule);
    }
}

/// Converts ContentAction to string.
fn action_to_string(action: ContentAction) -> &'static str {
    match action {
        ContentAction::Block => "block",
        ContentAction::Warn => "warn",
        ContentAction::Allow => "allow",
    }
}

/// Converts string to ContentAction.
fn string_to_action(s: &str) -> ContentAction {
    match s {
        "warn" => ContentAction::Warn,
        "allow" => ContentAction::Allow,
        _ => ContentAction::Block,
    }
}

/// Formats threshold as a human-readable string.
fn format_threshold(threshold: f32) -> &'static str {
    if threshold >= 0.85 {
        "Low"
    } else if threshold >= 0.6 {
        "Medium"
    } else {
        "High"
    }
}

/// Saves content rules to the database and notifies the proxy.
fn save_content_rules(state: &Signal<AppState>, content_rules: &Signal<ContentRuleSet>) {
    let state_ref = state.read();
    let Some(profile_id) = state_ref.selected_profile_id else {
        return;
    };

    let Ok(Some(profile)) = state_ref.db.get_profile(profile_id) else {
        return;
    };

    // Convert ContentRuleSet to JSON
    let content_rules_json = serde_json::to_value(content_rules.read().clone()).unwrap_or_default();

    let updated_profile = aegis_storage::NewProfile {
        name: profile.name,
        os_username: profile.os_username,
        time_rules: profile.time_rules,
        content_rules: content_rules_json,
        enabled: profile.enabled,
        sentiment_config: profile.sentiment_config,
    };

    if let Err(e) = state_ref.db.update_profile(profile_id, updated_profile) {
        tracing::error!("Failed to save content rules: {}", e);
        return;
    }

    tracing::info!("Content rules saved for profile {}", profile_id);

    // Notify the proxy to reload rules
    reload_rules_from_api(profile_id);
}

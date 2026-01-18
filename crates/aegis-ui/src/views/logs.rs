//! Activity logs view.

use chrono::NaiveDate;
use dioxus::prelude::*;

use aegis_core::classifier::Category;

use crate::state::AppState;

/// Logs view component.
#[component]
pub fn LogsView() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let mut events = use_signal(Vec::new);
    let mut loading = use_signal(|| true);
    let mut offset = use_signal(|| 0i64);
    let limit = 50i64;

    // Filter state
    let mut profile_filter = use_signal(|| None::<i64>);
    let mut action_filter = use_signal(|| None::<aegis_storage::Action>);
    let mut category_filter = use_signal(|| None::<Category>);
    let mut search_text = use_signal(String::new);
    let mut date_from = use_signal(|| None::<NaiveDate>);
    let mut date_to = use_signal(|| None::<NaiveDate>);

    let profiles = state.read().profiles.clone();

    // Load events function
    let mut load_events = move || {
        let state_ref = state.read();
        match state_ref.get_filtered_events(limit, offset()) {
            Ok(mut e) => {
                // Apply client-side filters
                let search = search_text().to_lowercase();
                let cat_filter = category_filter();
                let from = date_from();
                let to = date_to();

                e.retain(|event| {
                    // Search filter
                    if !search.is_empty() && !event.preview.to_lowercase().contains(&search) {
                        return false;
                    }
                    // Category filter
                    if let Some(cat) = cat_filter {
                        if event.category != Some(cat) {
                            return false;
                        }
                    }
                    // Date filters
                    if let Some(from_date) = from {
                        if event.created_at.date_naive() < from_date {
                            return false;
                        }
                    }
                    if let Some(to_date) = to {
                        if event.created_at.date_naive() > to_date {
                            return false;
                        }
                    }
                    true
                });
                events.set(e);
            }
            Err(e) => {
                drop(state_ref);
                state.write().set_error(e.to_string());
            }
        }
        loading.set(false);
    };

    // Load events on mount
    use_effect(move || {
        load_events();
    });

    let events_list = events();

    // Check if any filters are active
    let has_filters = action_filter().is_some()
        || category_filter().is_some()
        || !search_text().is_empty()
        || date_from().is_some()
        || date_to().is_some();

    rsx! {
        div {
            // Header
            div { class: "flex justify-between items-center mb-md",
                h1 { class: "text-lg font-bold", "Activity Logs" }
                button {
                    class: "btn btn-secondary",
                    onclick: move |_| {
                        // Export to CSV
                        if let Some(path) = directories::UserDirs::new()
                            .and_then(|d| d.document_dir().map(|p| p.join("aegis_logs.csv")))
                        {
                            let result = state.read().export_logs_csv(&path);
                            if let Err(e) = result {
                                state.write().set_error(format!("Export failed: {}", e));
                            } else {
                                state.write().set_success(format!("Exported to {}", path.display()));
                                let _ = open::that(&path);
                            }
                        }
                    },
                    "Export CSV"
                }
            }

            // Enhanced Filter Bar
            div { class: "filter-bar card mb-md",
                // Row 1: Main filters
                div { class: "filter-row",
                    // Profile filter
                    select {
                        class: "select",
                        onchange: move |evt| {
                            let profile_id = evt.value().parse::<i64>().ok();
                            profile_filter.set(profile_id);
                            offset.set(0);
                            load_events();
                        },
                        option { value: "", "All Profiles" }
                        for profile in profiles.iter() {
                            option { value: "{profile.id}", "{profile.name}" }
                        }
                    }

                    // Category filter
                    select {
                        class: "select",
                        onchange: move |evt| {
                            let category = match evt.value().as_str() {
                                "violence" => Some(Category::Violence),
                                "selfharm" => Some(Category::SelfHarm),
                                "adult" => Some(Category::Adult),
                                "jailbreak" => Some(Category::Jailbreak),
                                "hate" => Some(Category::Hate),
                                "illegal" => Some(Category::Illegal),
                                "profanity" => Some(Category::Profanity),
                                _ => None,
                            };
                            category_filter.set(category);
                            offset.set(0);
                            load_events();
                        },
                        option { value: "", "All Categories" }
                        option { value: "violence", "Violence" }
                        option { value: "selfharm", "Self-Harm" }
                        option { value: "adult", "Adult" }
                        option { value: "jailbreak", "Jailbreak" }
                        option { value: "hate", "Hate" }
                        option { value: "illegal", "Illegal" }
                        option { value: "profanity", "Profanity" }
                    }

                    // Action filter
                    select {
                        class: "select",
                        onchange: move |evt| {
                            let action = match evt.value().as_str() {
                                "allowed" => Some(aegis_storage::Action::Allowed),
                                "blocked" => Some(aegis_storage::Action::Blocked),
                                "flagged" => Some(aegis_storage::Action::Flagged),
                                _ => None,
                            };
                            action_filter.set(action);
                            state.write().log_filter.action = action;
                            offset.set(0);
                            load_events();
                        },
                        option { value: "", "All Actions" }
                        option { value: "allowed", "Allowed" }
                        option { value: "blocked", "Blocked" }
                        option { value: "flagged", "Flagged" }
                    }

                    // Clear filters button
                    if has_filters {
                        button {
                            class: "btn btn-secondary btn-sm",
                            onclick: move |_| {
                                action_filter.set(None);
                                category_filter.set(None);
                                search_text.set(String::new());
                                date_from.set(None);
                                date_to.set(None);
                                state.write().log_filter.action = None;
                                offset.set(0);
                                load_events();
                            },
                            "Clear Filters"
                        }
                    }
                }

                // Row 2: Search and date range
                div { class: "filter-row",
                    // Search input
                    div { class: "search-input-wrapper",
                        span { class: "search-icon", "🔍" }
                        input {
                            class: "input",
                            placeholder: "Search in previews...",
                            value: "{search_text}",
                            oninput: move |evt| {
                                search_text.set(evt.value());
                                offset.set(0);
                                load_events();
                            }
                        }
                    }

                    // Date from
                    div { class: "flex items-center gap-xs",
                        span { class: "text-sm text-muted", "From:" }
                        input {
                            r#type: "date",
                            value: date_from().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default(),
                            onchange: move |evt| {
                                let date = NaiveDate::parse_from_str(&evt.value(), "%Y-%m-%d").ok();
                                date_from.set(date);
                                offset.set(0);
                                load_events();
                            }
                        }
                    }

                    // Date to
                    div { class: "flex items-center gap-xs",
                        span { class: "text-sm text-muted", "To:" }
                        input {
                            r#type: "date",
                            value: date_to().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default(),
                            onchange: move |evt| {
                                let date = NaiveDate::parse_from_str(&evt.value(), "%Y-%m-%d").ok();
                                date_to.set(date);
                                offset.set(0);
                                load_events();
                            }
                        }
                    }
                }
            }

            // Logs table
            div { class: "card",
                if loading() {
                    div { class: "empty-state", "Loading..." }
                } else if events_list.is_empty() {
                    div { class: "empty-state",
                        p { class: "empty-state-text", "No activity logs" }
                        p { class: "empty-state-subtext", "Events will appear here when prompts are checked" }
                    }
                } else {
                    table { class: "table",
                        thead {
                            tr {
                                th { "Time" }
                                th { "Preview" }
                                th { "Category" }
                                th { "Action" }
                                th { "Source" }
                            }
                        }
                        tbody {
                            for event in events_list.iter() {
                                {
                                    let time_str = event.created_at.format("%Y-%m-%d %H:%M").to_string();
                                    let preview_str = truncate_text(&event.preview, 50);
                                    let category_name = event.category.map(|c| c.name().to_string());
                                    let (action_class, action_text) = match event.action {
                                        aegis_storage::Action::Allowed => ("tag-success", "Allowed"),
                                        aegis_storage::Action::Blocked => ("tag-error", "Blocked"),
                                        aegis_storage::Action::Flagged => ("tag-warning", "Flagged"),
                                    };
                                    let source_str = event.source.clone().unwrap_or_else(|| "-".to_string());

                                    rsx! {
                                        tr {
                                            td { "{time_str}" }
                                            td { style: "max-width: 300px; overflow: hidden; text-overflow: ellipsis;",
                                                "{preview_str}"
                                            }
                                            td {
                                                if let Some(ref cat_name) = category_name {
                                                    span { class: "tag tag-warning", "{cat_name}" }
                                                }
                                            }
                                            td {
                                                span { class: "tag {action_class}", "{action_text}" }
                                            }
                                            td { "{source_str}" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Pagination
                    {
                        let showing_start = offset() + 1;
                        let showing_end = offset() + events_list.len() as i64;
                        rsx! {
                            div { class: "flex justify-between items-center mt-md",
                                button {
                                    class: "btn btn-secondary btn-sm",
                                    disabled: offset() == 0,
                                    onclick: move |_| {
                                        let new_offset = (offset() - limit).max(0);
                                        offset.set(new_offset);
                                        load_events();
                                    },
                                    "Previous"
                                }
                                span { class: "text-sm text-muted", "Showing {showing_start} - {showing_end}" }
                                button {
                                    class: "btn btn-secondary btn-sm",
                                    disabled: events_list.len() < limit as usize,
                                    onclick: move |_| {
                                        let new_offset = offset() + limit;
                                        offset.set(new_offset);
                                        load_events();
                                    },
                                    "Next"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Truncates text to a maximum length.
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() > max_len {
        format!("{}...", &text[..max_len])
    } else {
        text.to_string()
    }
}

//! Activity logs view.

use dioxus::prelude::*;

use crate::state::AppState;

/// Logs view component.
#[component]
pub fn LogsView() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let mut events = use_signal(Vec::new);
    let mut loading = use_signal(|| true);
    let mut offset = use_signal(|| 0i64);
    let limit = 50i64;

    // Load events on mount
    use_effect(move || {
        let state_ref = state.read();
        match state_ref.get_filtered_events(limit, offset()) {
            Ok(e) => events.set(e),
            Err(e) => {
                drop(state_ref);
                state.write().set_error(e.to_string());
            }
        }
        loading.set(false);
    });

    let events_list = events();

    rsx! {
        div {
            // Header
            div { class: "flex justify-between items-center mb-lg",
                h1 { class: "text-lg font-bold", "Activity Logs" }
                div { class: "flex gap-sm",
                    // Filter by action
                    select {
                        class: "select",
                        onchange: move |evt| {
                            let action = match evt.value().as_str() {
                                "allowed" => Some(aegis_storage::Action::Allowed),
                                "blocked" => Some(aegis_storage::Action::Blocked),
                                "flagged" => Some(aegis_storage::Action::Flagged),
                                _ => None,
                            };
                            state.write().log_filter.action = action;
                            offset.set(0);
                            let result = state.read().get_filtered_events(limit, 0);
                            if let Ok(e) = result {
                                events.set(e);
                            }
                        },
                        option { value: "", "All Actions" }
                        option { value: "allowed", "Allowed" }
                        option { value: "blocked", "Blocked" }
                        option { value: "flagged", "Flagged" }
                    }
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
                                        let result = state.read().get_filtered_events(limit, new_offset);
                                        if let Ok(e) = result {
                                            events.set(e);
                                        }
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
                                        let result = state.read().get_filtered_events(limit, new_offset);
                                        if let Ok(e) = result {
                                            events.set(e);
                                        }
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

//! Flagged events for parental review.

use dioxus::prelude::*;

use crate::state::AppState;

/// Flagged events view component.
#[component]
pub fn FlaggedView() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let flagged_events = state.read().flagged_events.clone();
    let flagged_stats = state.read().flagged_stats.clone();

    rsx! {
        div {
            // Header
            div { class: "flex justify-between items-center mb-lg",
                div {
                    h1 { class: "text-lg font-bold", "Flagged for Review" }
                    if let Some(ref stats) = flagged_stats {
                        p { class: "text-sm text-muted",
                            "{stats.unacknowledged} unacknowledged, {stats.total} total"
                        }
                    }
                }
                if flagged_stats.as_ref().map(|s| s.unacknowledged > 0).unwrap_or(false) {
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| {
                            let result = state.write().acknowledge_all_flagged();
                            match result {
                                Ok(()) => state.write().set_success("All items acknowledged"),
                                Err(e) => state.write().set_error(e.to_string()),
                            }
                        },
                        "Acknowledge All"
                    }
                }
            }

            // Flagged items list
            div { class: "card",
                if flagged_events.is_empty() {
                    div { class: "empty-state",
                        p { class: "empty-state-text", "No flagged items" }
                        p { class: "empty-state-subtext", "Concerning prompts will appear here for your review" }
                    }
                } else {
                    for event in flagged_events.iter() {
                        {
                            let event_id = event.id;
                            let flag_type = event.flag_type.clone();
                            let confidence = event.confidence;
                            let content_snippet = event.content_snippet.clone();
                            let acknowledged = event.acknowledged;
                            let created_at = event.created_at.format("%Y-%m-%d %H:%M").to_string();

                            rsx! {
                                FlaggedItem {
                                    event_id: event_id,
                                    flag_type: flag_type,
                                    confidence: confidence,
                                    content_snippet: content_snippet,
                                    acknowledged: acknowledged,
                                    created_at: created_at,
                                    on_acknowledge: move |_| {
                                        let result = state.write().acknowledge_flagged(event_id);
                                        if let Err(e) = result {
                                            state.write().set_error(e.to_string());
                                        }
                                    },
                                    on_delete: move |_| {
                                        let result = state.write().delete_flagged(event_id);
                                        if let Err(e) = result {
                                            state.write().set_error(e.to_string());
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
}

/// Flagged item component.
#[component]
fn FlaggedItem(
    event_id: i64,
    flag_type: String,
    confidence: f32,
    content_snippet: String,
    acknowledged: bool,
    created_at: String,
    on_acknowledge: EventHandler<MouseEvent>,
    on_delete: EventHandler<MouseEvent>,
) -> Element {
    let severity_class = if confidence > 0.8 {
        "tag-error"
    } else if confidence > 0.5 {
        "tag-warning"
    } else {
        "tag-success"
    };

    rsx! {
        div { class: "activity-item", style: "flex-direction: column; align-items: flex-start; gap: 8px;",
            // Header row
            div { class: "flex w-full justify-between items-center",
                div { class: "flex gap-sm items-center",
                    span { class: "tag {severity_class}", "{flag_type}" }
                    if !acknowledged {
                        span { class: "tag tag-error", "NEW" }
                    }
                }
                span { class: "text-sm text-muted", "{created_at}" }
            }

            // Content
            div { class: "w-full",
                p { style: "word-break: break-word;", "{content_snippet}" }
                p { class: "text-sm text-muted mt-sm", "Confidence: {confidence:.0}%" }
            }

            // Actions
            div { class: "flex gap-sm mt-sm",
                if !acknowledged {
                    button {
                        class: "btn btn-primary btn-sm",
                        onclick: move |evt| on_acknowledge.call(evt),
                        "Acknowledge"
                    }
                }
                button {
                    class: "btn btn-secondary btn-sm",
                    onclick: move |evt| on_delete.call(evt),
                    "Delete"
                }
            }
        }
    }
}

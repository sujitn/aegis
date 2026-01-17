//! System logs viewer.

use std::collections::VecDeque;
use std::path::PathBuf;

use directories::ProjectDirs;
use dioxus::prelude::*;

/// Log level for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    pub fn css_class(&self) -> &'static str {
        match self {
            LogLevel::Trace => "text-muted",
            LogLevel::Debug => "",
            LogLevel::Info => "tag-success",
            LogLevel::Warn => "tag-warning",
            LogLevel::Error => "tag-error",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_uppercase().as_str() {
            "TRACE" => Some(LogLevel::Trace),
            "DEBUG" => Some(LogLevel::Debug),
            "INFO" => Some(LogLevel::Info),
            "WARN" | "WARNING" => Some(LogLevel::Warn),
            "ERROR" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

/// A parsed log entry.
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
}

impl LogEntry {
    pub fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        let mut parts = line.splitn(2, char::is_whitespace);
        let timestamp = parts.next()?.to_string();

        if !timestamp.contains('T') && !timestamp.contains('-') {
            return Some(LogEntry {
                timestamp: String::new(),
                level: LogLevel::Info,
                target: String::new(),
                message: line.to_string(),
            });
        }

        let rest = parts.next()?.trim();
        let mut parts = rest.splitn(2, char::is_whitespace);
        let level_str = parts.next()?;
        let level = LogLevel::parse(level_str).unwrap_or(LogLevel::Info);

        let rest = parts.next().unwrap_or("").trim();
        let (target, message) = if let Some(colon_pos) = rest.find(':') {
            (
                rest[..colon_pos].trim().to_string(),
                rest[colon_pos + 1..].trim().to_string(),
            )
        } else {
            (String::new(), rest.to_string())
        };

        Some(LogEntry {
            timestamp,
            level,
            target,
            message,
        })
    }
}

/// Find the most recent log file.
fn find_log_file() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "aegis", "Aegis")?;
    let log_dir = dirs.data_dir().join("logs");

    if !log_dir.exists() {
        return None;
    }

    std::fs::read_dir(&log_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "log")
                .unwrap_or(false)
        })
        .max_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()))
        .map(|e| e.path())
}

/// Load log entries from file.
fn load_logs(path: &PathBuf) -> VecDeque<LogEntry> {
    let mut entries = VecDeque::with_capacity(500);

    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            if let Some(entry) = LogEntry::parse(line) {
                entries.push_back(entry);
                if entries.len() > 500 {
                    entries.pop_front();
                }
            }
        }
    }

    entries
}

/// System logs view component.
#[component]
pub fn SystemLogsView() -> Element {
    let mut entries = use_signal(VecDeque::new);
    let mut search = use_signal(String::new);
    let mut show_info = use_signal(|| true);
    let mut show_warn = use_signal(|| true);
    let mut show_error = use_signal(|| true);
    let mut show_debug = use_signal(|| true);
    let log_path = use_signal(find_log_file);

    // Load logs on mount
    use_effect(move || {
        if let Some(path) = log_path() {
            entries.set(load_logs(&path));
        }
    });

    let filtered: Vec<_> = entries()
        .iter()
        .filter(|e| {
            let level_ok = match e.level {
                LogLevel::Info => show_info(),
                LogLevel::Warn => show_warn(),
                LogLevel::Error => show_error(),
                LogLevel::Debug => show_debug(),
                LogLevel::Trace => false,
            };

            if !level_ok {
                return false;
            }

            if !search().is_empty() {
                let q = search().to_lowercase();
                if !e.message.to_lowercase().contains(&q)
                    && !e.target.to_lowercase().contains(&q)
                {
                    return false;
                }
            }

            true
        })
        .cloned()
        .collect();

    let log_path_display = log_path().map(|p| p.display().to_string());
    let filtered_count = filtered.len();
    let total_count = entries().len();

    rsx! {
        div {
            // Header
            div { class: "flex justify-between items-center mb-lg",
                h1 { class: "text-lg font-bold", "System Logs" }
                div { class: "flex gap-sm",
                    button {
                        class: "btn btn-secondary btn-sm",
                        onclick: move |_| {
                            if let Some(path) = log_path() {
                                entries.set(load_logs(&path));
                            }
                        },
                        "Refresh"
                    }
                    button {
                        class: "btn btn-secondary btn-sm",
                        onclick: move |_| {
                            if let Some(dirs) = ProjectDirs::from("", "aegis", "Aegis") {
                                let log_dir = dirs.data_dir().join("logs");
                                let _ = open::that(&log_dir);
                            }
                        },
                        "Open Log Folder"
                    }
                }
            }

            // Filters
            div { class: "flex gap-md items-center mb-md",
                input {
                    class: "input",
                    style: "width: 200px;",
                    placeholder: "Search logs...",
                    value: "{search}",
                    oninput: move |evt| search.set(evt.value())
                }

                label { class: "checkbox",
                    input {
                        r#type: "checkbox",
                        checked: "{show_error}",
                        onchange: move |evt| show_error.set(evt.checked())
                    }
                    "Error"
                }
                label { class: "checkbox",
                    input {
                        r#type: "checkbox",
                        checked: "{show_warn}",
                        onchange: move |evt| show_warn.set(evt.checked())
                    }
                    "Warn"
                }
                label { class: "checkbox",
                    input {
                        r#type: "checkbox",
                        checked: "{show_info}",
                        onchange: move |evt| show_info.set(evt.checked())
                    }
                    "Info"
                }
                label { class: "checkbox",
                    input {
                        r#type: "checkbox",
                        checked: "{show_debug}",
                        onchange: move |evt| show_debug.set(evt.checked())
                    }
                    "Debug"
                }
            }

            // Log file path
            if let Some(ref path_str) = log_path_display {
                p { class: "text-sm text-muted mb-sm", "Log file: {path_str}" }
            }

            // Log entries
            div { class: "card", style: "max-height: 500px; overflow-y: auto; font-family: monospace; font-size: 11px;",
                if filtered.is_empty() {
                    div { class: "empty-state",
                        p { class: "empty-state-text", "No log entries" }
                    }
                } else {
                    for entry in filtered.iter().rev().take(200) {
                        {
                            let short_time = if entry.timestamp.len() > 19 {
                                entry.timestamp[11..19].to_string()
                            } else {
                                entry.timestamp.clone()
                            };
                            let level_class = entry.level.css_class();
                            let level_str = entry.level.as_str();
                            let target_str = truncate_target(&entry.target);
                            let has_target = !entry.target.is_empty();
                            let message = entry.message.clone();

                            rsx! {
                                div { class: "flex gap-sm", style: "padding: 2px 0; border-bottom: 1px solid var(--aegis-slate-700);",
                                    span { class: "text-muted", style: "width: 60px;", "{short_time}" }
                                    span { class: "tag {level_class}", style: "width: 50px; text-align: center;", "{level_str}" }
                                    if has_target {
                                        span { class: "text-muted", style: "width: 150px; overflow: hidden; text-overflow: ellipsis;",
                                            "{target_str}"
                                        }
                                    }
                                    span { style: "flex: 1;", "{message}" }
                                }
                            }
                        }
                    }
                }
            }

            p { class: "text-sm text-muted mt-sm",
                "Showing {filtered_count} of {total_count} entries"
            }
        }
    }
}

/// Truncates target if too long.
fn truncate_target(s: &str) -> String {
    if s.len() > 25 {
        format!("...{}", &s[s.len() - 22..])
    } else {
        s.to_string()
    }
}

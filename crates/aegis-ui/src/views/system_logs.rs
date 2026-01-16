//! System logs viewer (application logs from file).

use std::collections::VecDeque;
use std::path::PathBuf;

use directories::ProjectDirs;
use eframe::egui::{self, Color32, RichText, ScrollArea};

/// Log level for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
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

    pub fn color(&self) -> Color32 {
        match self {
            LogLevel::Trace => Color32::GRAY,
            LogLevel::Debug => Color32::from_rgb(100, 149, 237), // Cornflower blue
            LogLevel::Info => Color32::from_rgb(34, 139, 34),    // Forest green
            LogLevel::Warn => Color32::from_rgb(255, 165, 0),    // Orange
            LogLevel::Error => Color32::from_rgb(220, 20, 60),   // Crimson
        }
    }

    pub fn parse_level(s: &str) -> Option<Self> {
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
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    pub raw: String,
}

impl LogEntry {
    /// Parse a log line into a LogEntry.
    pub fn parse(line: &str) -> Option<Self> {
        // Expected format: 2024-01-15T10:30:45.123Z INFO aegis_proxy: Message
        // Or: 2024-01-15T10:30:45.123456Z  INFO aegis_proxy::handler: Message
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Try to extract timestamp (ISO format at start)
        let mut parts = line.splitn(2, char::is_whitespace);
        let timestamp = parts.next()?.to_string();

        // Skip if doesn't look like a timestamp
        if !timestamp.contains('T') && !timestamp.contains('-') {
            return Some(LogEntry {
                timestamp: String::new(),
                level: LogLevel::Info,
                target: String::new(),
                message: line.to_string(),
                raw: line.to_string(),
            });
        }

        let rest = parts.next()?.trim();

        // Extract level
        let mut parts = rest.splitn(2, char::is_whitespace);
        let level_str = parts.next()?;
        let level = LogLevel::parse_level(level_str).unwrap_or(LogLevel::Info);

        let rest = parts.next().unwrap_or("").trim();

        // Extract target and message (target ends with ':')
        let (target, message) = if let Some(colon_pos) = rest.find(':') {
            let target = rest[..colon_pos].trim().to_string();
            let message = rest[colon_pos + 1..].trim().to_string();
            (target, message)
        } else {
            (String::new(), rest.to_string())
        };

        Some(LogEntry {
            timestamp,
            level,
            target,
            message,
            raw: line.to_string(),
        })
    }
}

/// State for the system logs viewer.
pub struct SystemLogsState {
    /// Loaded log entries.
    pub entries: VecDeque<LogEntry>,
    /// Maximum entries to keep in memory.
    pub max_entries: usize,
    /// Filter by minimum log level.
    pub min_level: LogLevel,
    /// Search query.
    pub search_query: String,
    /// Auto-scroll to bottom.
    pub auto_scroll: bool,
    /// Last loaded file position.
    pub last_position: u64,
    /// Log file path.
    pub log_path: Option<PathBuf>,
    /// Show level filter checkboxes.
    pub show_trace: bool,
    pub show_debug: bool,
    pub show_info: bool,
    pub show_warn: bool,
    pub show_error: bool,
}

impl Default for SystemLogsState {
    fn default() -> Self {
        Self {
            entries: VecDeque::with_capacity(1000),
            max_entries: 1000,
            min_level: LogLevel::Info,
            search_query: String::new(),
            auto_scroll: true,
            last_position: 0,
            log_path: Self::find_log_file(),
            show_trace: false,
            show_debug: true,
            show_info: true,
            show_warn: true,
            show_error: true,
        }
    }
}

impl SystemLogsState {
    /// Find the most recent log file.
    fn find_log_file() -> Option<PathBuf> {
        let dirs = ProjectDirs::from("", "aegis", "Aegis")?;
        let log_dir = dirs.data_dir().join("logs");

        if !log_dir.exists() {
            return None;
        }

        // Find the most recent log file
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

    /// Refresh log entries from file.
    pub fn refresh(&mut self) {
        let Some(path) = &self.log_path else {
            return;
        };

        // Try to read new content
        if let Ok(content) = std::fs::read_to_string(path) {
            self.entries.clear();

            for line in content.lines() {
                if let Some(entry) = LogEntry::parse(line) {
                    self.entries.push_back(entry);
                    if self.entries.len() > self.max_entries {
                        self.entries.pop_front();
                    }
                }
            }
        }
    }

    /// Check if an entry matches the current filters.
    pub fn matches_filter(&self, entry: &LogEntry) -> bool {
        // Level filter
        let level_ok = match entry.level {
            LogLevel::Trace => self.show_trace,
            LogLevel::Debug => self.show_debug,
            LogLevel::Info => self.show_info,
            LogLevel::Warn => self.show_warn,
            LogLevel::Error => self.show_error,
        };

        if !level_ok {
            return false;
        }

        // Search filter
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            if !entry.message.to_lowercase().contains(&query)
                && !entry.target.to_lowercase().contains(&query)
            {
                return false;
            }
        }

        true
    }
}

/// Renders the system logs view.
pub fn render(ui: &mut egui::Ui, state: &mut SystemLogsState) {
    // Header
    ui.horizontal(|ui| {
        ui.heading("System Logs");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Open Log Folder").clicked() {
                if let Some(dirs) = ProjectDirs::from("", "aegis", "Aegis") {
                    let log_dir = dirs.data_dir().join("logs");
                    let _ = open::that(&log_dir);
                }
            }

            if ui.button("Refresh").clicked() {
                state.refresh();
            }

            ui.checkbox(&mut state.auto_scroll, "Auto-scroll");
        });
    });

    ui.add_space(8.0);

    // Filters
    ui.horizontal(|ui| {
        ui.label("Search:");
        let response = ui.add(
            egui::TextEdit::singleline(&mut state.search_query)
                .hint_text("Filter logs...")
                .desired_width(200.0),
        );
        if response.changed() {
            // Filter will be applied on render
        }

        ui.add_space(16.0);

        ui.label("Levels:");
        ui.checkbox(&mut state.show_error, "Error");
        ui.checkbox(&mut state.show_warn, "Warn");
        ui.checkbox(&mut state.show_info, "Info");
        ui.checkbox(&mut state.show_debug, "Debug");
        ui.checkbox(&mut state.show_trace, "Trace");
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    // Log file info
    if let Some(ref path) = state.log_path {
        ui.label(
            RichText::new(format!("Log file: {}", path.display()))
                .size(10.0)
                .weak(),
        );
    } else {
        ui.label(RichText::new("No log file found").size(10.0).weak());
    }

    ui.add_space(4.0);

    // Log entries
    let filtered_entries: Vec<_> = state
        .entries
        .iter()
        .filter(|e| state.matches_filter(e))
        .collect();

    let row_height = 18.0;
    let total_rows = filtered_entries.len();

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(state.auto_scroll)
        .show_rows(ui, row_height, total_rows, |ui, row_range| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

            for row in row_range {
                if let Some(entry) = filtered_entries.get(row) {
                    render_log_entry(ui, entry);
                }
            }
        });

    // Stats
    ui.add_space(4.0);
    ui.label(
        RichText::new(format!(
            "Showing {} of {} entries",
            filtered_entries.len(),
            state.entries.len()
        ))
        .size(10.0)
        .weak(),
    );
}

/// Renders a single log entry.
fn render_log_entry(ui: &mut egui::Ui, entry: &LogEntry) {
    ui.horizontal(|ui| {
        // Timestamp
        if !entry.timestamp.is_empty() {
            // Shorten timestamp for display
            let short_time = if entry.timestamp.len() > 19 {
                &entry.timestamp[11..19] // Just HH:MM:SS
            } else {
                &entry.timestamp
            };
            ui.label(RichText::new(short_time).size(11.0).weak());
        }

        // Level badge
        let level_text = entry.level.as_str();
        let level_color = entry.level.color();

        egui::Frame::none()
            .fill(level_color.gamma_multiply(0.15))
            .rounding(2.0)
            .inner_margin(egui::vec2(4.0, 1.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(format!("{:5}", level_text))
                        .size(10.0)
                        .color(level_color),
                );
            });

        // Target
        if !entry.target.is_empty() {
            let short_target = if entry.target.len() > 25 {
                format!("...{}", &entry.target[entry.target.len() - 22..])
            } else {
                entry.target.clone()
            };
            ui.label(RichText::new(short_target).size(10.0).color(Color32::GRAY));
        }

        // Message
        ui.label(RichText::new(&entry.message).size(11.0));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::parse_level("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::parse_level("info"), Some(LogLevel::Info));
        assert_eq!(LogLevel::parse_level("WARN"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::parse_level("WARNING"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::parse_level("ERROR"), Some(LogLevel::Error));
        assert_eq!(LogLevel::parse_level("DEBUG"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::parse_level("TRACE"), Some(LogLevel::Trace));
        assert_eq!(LogLevel::parse_level("unknown"), None);
    }

    #[test]
    fn test_parse_log_entry() {
        let line = "2024-01-15T10:30:45.123Z INFO aegis_proxy: Test message";
        let entry = LogEntry::parse(line).unwrap();

        assert_eq!(entry.timestamp, "2024-01-15T10:30:45.123Z");
        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.target, "aegis_proxy");
        assert_eq!(entry.message, "Test message");
    }

    #[test]
    fn test_parse_log_entry_warn() {
        let line = "2024-01-15T10:30:45.123Z WARN aegis: Warning message";
        let entry = LogEntry::parse(line).unwrap();

        assert_eq!(entry.level, LogLevel::Warn);
        assert_eq!(entry.message, "Warning message");
    }

    #[test]
    fn test_system_logs_state_default() {
        let state = SystemLogsState::default();
        assert!(state.entries.is_empty());
        assert!(state.auto_scroll);
        assert!(state.show_info);
        assert!(state.show_warn);
        assert!(state.show_error);
    }
}

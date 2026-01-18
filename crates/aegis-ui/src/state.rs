//! Application state management for Dioxus.

use std::sync::Arc;
use std::time::{Duration, Instant};

use aegis_core::auth::{AuthManager, SessionToken, SESSION_TIMEOUT};
use aegis_core::protection::{PauseDuration, ProtectionManager};
use aegis_proxy::FilteringState;
use aegis_storage::{
    DailyStats, Database, Event, FlaggedEvent, FlaggedEventStats, PauseDuration as StoragePauseDuration,
    Profile, StateManager,
};
use chrono::{DateTime, Utc};
use chrono::{Local, NaiveDate};

use crate::error::{Result, UiError};

/// Protection status of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProtectionStatus {
    /// Actively filtering.
    #[default]
    Active,
    /// Temporarily paused.
    Paused,
    /// Completely disabled.
    Disabled,
}

impl ProtectionStatus {
    /// Returns display text.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Paused => "Paused",
            Self::Disabled => "Disabled",
        }
    }

    /// Returns the CSS class for this status.
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Disabled => "disabled",
        }
    }
}

/// Interception mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InterceptionMode {
    /// MITM proxy for all apps.
    #[default]
    Proxy,
}

impl InterceptionMode {
    /// Returns display text.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proxy => "Proxy",
        }
    }
}

/// Current navigation view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    /// Login screen (when not authenticated).
    Login,
    /// First-run setup wizard.
    Setup,
    /// Main dashboard home.
    #[default]
    Dashboard,
    /// Profile management.
    Profiles,
    /// Rule configuration for selected profile.
    Rules,
    /// Event logs (activity).
    Logs,
    /// Flagged items for parental review.
    Flagged,
    /// System logs (application logs from file).
    SystemLogs,
    /// Application settings.
    Settings,
}

/// Sub-tabs for rules view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RulesTab {
    #[default]
    Time,
    Content,
    Community,
}

/// Filter options for logs.
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    /// Filter by profile name.
    pub profile: Option<String>,
    /// Filter by action.
    pub action: Option<aegis_storage::Action>,
    /// Filter by category.
    pub category: Option<aegis_core::classifier::Category>,
    /// Filter by date range start.
    pub date_from: Option<NaiveDate>,
    /// Filter by date range end.
    pub date_to: Option<NaiveDate>,
    /// Search text.
    pub search: String,
}

/// Application state for the dashboard.
/// This state is shared via Dioxus context and must be Clone.
#[derive(Clone)]
pub struct AppState {
    /// Database connection (Arc for cloning).
    pub db: Arc<Database>,

    /// Authentication manager (Arc for cloning).
    pub auth: Arc<AuthManager>,

    /// Protection manager for pause/resume/disable.
    pub protection: ProtectionManager,

    /// Centralized state manager for cross-process state (F032).
    pub state_manager: StateManager,

    /// Current session token.
    pub session: Option<SessionToken>,

    /// Last activity time for session timeout (wrapped in Arc for cloning).
    last_activity: Arc<std::sync::Mutex<Instant>>,

    /// Current view.
    pub view: View,

    /// Cached protection status for UI reactivity.
    /// This is updated whenever protection state changes.
    pub cached_protection_status: ProtectionStatus,

    /// Cached pause expiry time (from database).
    pub cached_pause_until: Option<DateTime<Utc>>,

    /// Interception mode.
    pub interception_mode: InterceptionMode,

    /// Selected profile ID for rules view.
    pub selected_profile_id: Option<i64>,

    /// Current rules tab.
    pub rules_tab: RulesTab,

    /// Log filter settings.
    pub log_filter: LogFilter,

    /// Cached today's stats.
    pub today_stats: Option<DailyStats>,

    /// Cached recent events.
    pub recent_events: Vec<Event>,

    /// Cached flagged events for parental review.
    pub flagged_events: Vec<FlaggedEvent>,

    /// Cached flagged event statistics.
    pub flagged_stats: Option<FlaggedEventStats>,

    /// Cached profiles list.
    pub profiles: Vec<Profile>,

    /// Error message to display.
    pub error_message: Option<String>,

    /// Success message to display.
    pub success_message: Option<String>,

    /// Password input for login.
    pub password_input: String,

    /// New password input for change password.
    pub new_password_input: String,

    /// Confirm password input.
    pub confirm_password_input: String,

    /// Whether this is first-time setup (no password set).
    pub is_first_setup: bool,

    /// Optional filtering state for live rule updates.
    /// When set, rule changes in the UI are immediately applied to the proxy.
    pub filtering_state: Option<FilteringState>,
}

impl AppState {
    /// Creates a new application state.
    pub fn new(db: Database) -> Self {
        Self::with_filtering_state(db, None)
    }

    /// Creates a new application state with an optional filtering state.
    ///
    /// If `filtering_state` is provided, rule changes made in the UI will be
    /// immediately applied to the running proxy.
    pub fn with_filtering_state(db: Database, filtering_state: Option<FilteringState>) -> Self {
        let db_arc = Arc::new(db);
        let auth = AuthManager::new();
        let protection = ProtectionManager::new();
        let state_manager = StateManager::new(db_arc.clone(), "dashboard");
        let is_first_setup = !db_arc.is_auth_setup().unwrap_or(false);

        // Get initial protection status from database (F032)
        let (initial_protection_status, initial_pause_until) = state_manager
            .get_protection_state()
            .map(|ps| {
                let status = if ps.is_disabled() {
                    ProtectionStatus::Disabled
                } else if ps.is_paused() {
                    ProtectionStatus::Paused
                } else {
                    ProtectionStatus::Active
                };
                let pause_until = ps.pause_until.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                });
                (status, pause_until)
            })
            .unwrap_or((ProtectionStatus::Active, None));

        // Start with Setup view if first run, otherwise Login
        let initial_view = if is_first_setup {
            View::Setup
        } else {
            View::Login
        };

        Self {
            db: db_arc,
            auth: Arc::new(auth),
            protection,
            state_manager,
            session: None,
            last_activity: Arc::new(std::sync::Mutex::new(Instant::now())),
            view: initial_view,
            cached_protection_status: initial_protection_status,
            cached_pause_until: initial_pause_until,
            interception_mode: InterceptionMode::Proxy,
            selected_profile_id: None,
            rules_tab: RulesTab::Time,
            log_filter: LogFilter::default(),
            today_stats: None,
            recent_events: Vec::new(),
            flagged_events: Vec::new(),
            flagged_stats: None,
            profiles: Vec::new(),
            error_message: None,
            success_message: None,
            password_input: String::new(),
            new_password_input: String::new(),
            confirm_password_input: String::new(),
            is_first_setup,
            filtering_state,
        }
    }

    /// Creates state with in-memory database (for testing).
    pub fn in_memory() -> Result<Self> {
        let db = Database::in_memory()?;
        Ok(Self::new(db))
    }

    /// Checks if session is valid and not expired.
    pub fn is_authenticated(&self) -> bool {
        if let Some(ref token) = self.session {
            if let Ok(last) = self.last_activity.lock() {
                if last.elapsed() > SESSION_TIMEOUT {
                    return false;
                }
            }
            self.auth.is_session_valid(token)
        } else {
            false
        }
    }

    /// Updates last activity time.
    pub fn touch_activity(&mut self) {
        if let Ok(mut last) = self.last_activity.lock() {
            *last = Instant::now();
        }
    }

    /// Attempts to login with password.
    pub fn login(&mut self, password: &str) -> Result<()> {
        let hash = self.db.get_password_hash()?;

        if self.auth.verify_password(password, &hash)? {
            let token = self.auth.create_session();
            self.session = Some(token);
            self.touch_activity();
            self.db.update_last_login()?;
            self.view = View::Dashboard;
            self.password_input.clear();
            self.refresh_data()?;
            Ok(())
        } else {
            Err(UiError::InvalidInput("Incorrect password".into()))
        }
    }

    /// Sets up initial password (first run).
    pub fn setup_password(&mut self, password: &str) -> Result<()> {
        let hash = self.auth.hash_password(password)?;
        self.db.set_password_hash(&hash)?;
        self.is_first_setup = false;

        // Auto-login after setup
        let token = self.auth.create_session();
        self.session = Some(token);
        self.touch_activity();
        self.view = View::Dashboard;
        self.password_input.clear();

        Ok(())
    }

    /// Changes password.
    pub fn change_password(&mut self, current: &str, new_password: &str) -> Result<()> {
        // Verify current password
        let hash = self.db.get_password_hash()?;
        if !self.auth.verify_password(current, &hash)? {
            return Err(UiError::InvalidInput(
                "Current password is incorrect".into(),
            ));
        }

        // Hash and save new password
        let new_hash = self.auth.hash_password(new_password)?;
        self.db.set_password_hash(&new_hash)?;

        // Invalidate all sessions and re-login
        self.auth.logout_all();
        let token = self.auth.create_session();
        self.session = Some(token);

        Ok(())
    }

    /// Locks the dashboard (requires re-authentication).
    pub fn lock(&mut self) {
        if let Some(ref token) = self.session {
            self.auth.logout(token);
        }
        self.session = None;
        self.view = View::Login;
        self.password_input.clear();
    }

    /// Logs out completely.
    pub fn logout(&mut self) {
        self.lock();
    }

    /// Refreshes cached data from database.
    pub fn refresh_data(&mut self) -> Result<()> {
        // Load today's stats
        let today = Local::now().date_naive();
        self.today_stats = self.db.get_stats(today)?;

        // Load recent events
        self.recent_events = self.db.get_recent_events(10, 0)?;

        // Load flagged events
        self.flagged_events = self.db.get_recent_flagged_events(50, 0)?;
        self.flagged_stats = self.db.get_flagged_event_stats().ok();

        // Load profiles
        self.profiles = self.load_profiles()?;

        Ok(())
    }

    /// Returns the count of unacknowledged flagged events.
    pub fn unacknowledged_flagged_count(&self) -> i64 {
        self.flagged_stats
            .as_ref()
            .map(|s| s.unacknowledged)
            .unwrap_or(0)
    }

    /// Acknowledges a flagged event.
    pub fn acknowledge_flagged(&mut self, id: i64) -> Result<()> {
        self.db.acknowledge_flagged_event(id)?;
        // Refresh flagged events
        self.flagged_events = self.db.get_recent_flagged_events(50, 0)?;
        self.flagged_stats = self.db.get_flagged_event_stats().ok();
        Ok(())
    }

    /// Acknowledges all flagged events.
    pub fn acknowledge_all_flagged(&mut self) -> Result<()> {
        let ids: Vec<i64> = self
            .flagged_events
            .iter()
            .filter(|e| !e.acknowledged)
            .map(|e| e.id)
            .collect();
        if !ids.is_empty() {
            self.db.acknowledge_flagged_events(&ids)?;
        }
        // Refresh flagged events
        self.flagged_events = self.db.get_recent_flagged_events(50, 0)?;
        self.flagged_stats = self.db.get_flagged_event_stats().ok();
        Ok(())
    }

    /// Deletes a flagged event.
    pub fn delete_flagged(&mut self, id: i64) -> Result<()> {
        self.db.delete_flagged_event(id)?;
        // Refresh flagged events
        self.flagged_events = self.db.get_recent_flagged_events(50, 0)?;
        self.flagged_stats = self.db.get_flagged_event_stats().ok();
        Ok(())
    }

    /// Loads profiles from database.
    fn load_profiles(&self) -> Result<Vec<Profile>> {
        self.db.get_all_profiles().map_err(UiError::Storage)
    }

    /// Gets filtered events for logs view.
    pub fn get_filtered_events(&self, limit: i64, offset: i64) -> Result<Vec<Event>> {
        // For now, just get recent events
        // Full filtering would require additional database queries
        if let Some(action) = self.log_filter.action {
            Ok(self.db.get_events_by_action(action, limit, offset)?)
        } else {
            Ok(self.db.get_recent_events(limit, offset)?)
        }
    }

    /// Exports logs to CSV.
    pub fn export_logs_csv(&self, path: &std::path::Path) -> Result<()> {
        let events = self.db.get_recent_events(10000, 0)?;

        let mut writer = csv::Writer::from_path(path)?;

        writer.write_record(["Timestamp", "Preview", "Category", "Action", "Source"])?;

        for event in events {
            writer.write_record([
                event.created_at.to_string(),
                event.preview,
                event
                    .category
                    .map(|c| c.name().to_string())
                    .unwrap_or_default(),
                event.action.as_str().to_string(),
                event.source.unwrap_or_default(),
            ])?;
        }

        writer.flush()?;
        Ok(())
    }

    // ==================== Protection Methods ====================

    /// Returns the current protection status for UI display.
    /// Uses the cached value for reactivity.
    pub fn protection_status(&self) -> ProtectionStatus {
        self.cached_protection_status
    }

    /// Syncs the cached protection status with the centralized state (F032).
    /// Reads from database via StateManager.
    fn sync_protection_status(&mut self) {
        // Read from centralized state manager (database)
        if let Ok(ps) = self.state_manager.get_protection_state() {
            self.cached_protection_status = if ps.is_disabled() {
                ProtectionStatus::Disabled
            } else if ps.is_paused() {
                ProtectionStatus::Paused
            } else {
                ProtectionStatus::Active
            };

            // Update pause expiry
            self.cached_pause_until = ps.pause_until.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });

            // Also sync local ProtectionManager state (for consistency)
            if self.cached_protection_status == ProtectionStatus::Active {
                let _ = self.protection.resume();
            }
        }
    }

    /// Refreshes protection status from the database.
    /// Call this to pick up changes made by other processes.
    pub fn refresh_protection_status(&mut self) {
        self.sync_protection_status();
    }

    /// Returns remaining pause time, if any.
    /// Uses the cached pause expiry from the database.
    pub fn pause_remaining(&self) -> Option<Duration> {
        self.cached_pause_until.and_then(|until| {
            let now = Utc::now();
            if until > now {
                let diff = until - now;
                Some(Duration::from_secs(diff.num_seconds().max(0) as u64))
            } else {
                None
            }
        })
    }

    /// Formats the remaining pause time as a human-readable string.
    pub fn pause_remaining_str(&self) -> Option<String> {
        self.pause_remaining().map(|d| {
            let total_secs = d.as_secs();
            let hours = total_secs / 3600;
            let mins = (total_secs % 3600) / 60;
            let secs = total_secs % 60;

            if hours > 0 {
                format!("{}h {}m", hours, mins)
            } else if mins > 0 {
                format!("{}m {}s", mins, secs)
            } else {
                format!("{}s", secs)
            }
        })
    }

    /// Pauses protection for the specified duration.
    /// Requires an authenticated session.
    /// Persists to database and calls API to notify proxy.
    pub fn pause_protection(&mut self, duration: PauseDuration) -> Result<()> {
        let session = self.session.as_ref().ok_or(UiError::Auth(
            aegis_core::auth::AuthError::SessionInvalid,
        ))?;

        // Convert aegis_core PauseDuration to aegis_storage PauseDuration
        let storage_duration = match duration {
            PauseDuration::Minutes(m) => StoragePauseDuration::Minutes(m),
            PauseDuration::Hours(h) => StoragePauseDuration::Hours(h),
            PauseDuration::Indefinite => StoragePauseDuration::Indefinite,
        };

        // Persist to database via StateManager (F032)
        self.state_manager
            .pause_protection(storage_duration)
            .map_err(|e| UiError::InvalidInput(format!("Failed to pause: {}", e)))?;

        // Also update local state for immediate UI feedback
        let _ = self.protection.pause(duration.clone(), session, &self.auth);

        // Call API to notify proxy (in case it's in a separate process)
        let (duration_type, duration_value) = match duration {
            PauseDuration::Minutes(m) => ("minutes", m),
            PauseDuration::Hours(h) => ("hours", h),
            PauseDuration::Indefinite => ("indefinite", 0),
        };

        let session_token = session.as_str().to_string();
        let _ = std::thread::spawn(move || {
            let client = reqwest::blocking::Client::new();
            let _ = client
                .post("http://127.0.0.1:48765/api/protection/pause")
                .json(&serde_json::json!({
                    "session_token": session_token,
                    "duration_type": duration_type,
                    "duration_value": duration_value
                }))
                .send();
        });

        self.sync_protection_status();
        Ok(())
    }

    /// Resumes protection immediately.
    /// Does not require authentication.
    /// Persists to database and calls API to notify proxy.
    pub fn resume_protection(&mut self) {
        // Persist to database via StateManager (F032)
        if let Err(e) = self.state_manager.resume_protection() {
            tracing::warn!("Failed to resume protection in database: {}", e);
        }

        // Also update local state
        let _ = self.protection.resume();

        // Call API to notify proxy (in case it's in a separate process)
        let _ = std::thread::spawn(|| {
            let client = reqwest::blocking::Client::new();
            let _ = client
                .post("http://127.0.0.1:48765/api/protection/resume")
                .json(&serde_json::json!({}))
                .send();
        });

        self.sync_protection_status();
    }

    /// Disables protection completely.
    /// Requires an authenticated session.
    /// Persists to database and calls API to notify proxy.
    pub fn disable_protection(&mut self) -> Result<()> {
        let session = self.session.as_ref().ok_or(UiError::Auth(
            aegis_core::auth::AuthError::SessionInvalid,
        ))?;

        // Persist to database via StateManager (F032)
        self.state_manager
            .disable_protection()
            .map_err(|e| UiError::InvalidInput(format!("Failed to disable: {}", e)))?;

        // Also update local state
        let _ = self.protection.disable(session, &self.auth);

        // Call API to notify proxy (in case it's in a separate process)
        let session_token = session.as_str().to_string();
        let _ = std::thread::spawn(move || {
            let client = reqwest::blocking::Client::new();
            let _ = client
                .post("http://127.0.0.1:48765/api/protection/pause")
                .json(&serde_json::json!({
                    "session_token": session_token,
                    "duration_type": "indefinite",
                    "duration_value": 0
                }))
                .send();
        });

        self.sync_protection_status();
        Ok(())
    }

    // ==================== Message Methods ====================

    /// Clears message after display.
    pub fn clear_messages(&mut self) {
        self.error_message = None;
        self.success_message = None;
    }

    /// Sets an error message.
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error_message = Some(msg.into());
        self.success_message = None;
    }

    /// Sets a success message.
    pub fn set_success(&mut self, msg: impl Into<String>) {
        self.success_message = Some(msg.into());
        self.error_message = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protection_status() {
        assert_eq!(ProtectionStatus::Active.as_str(), "Active");
        assert_eq!(ProtectionStatus::Paused.as_str(), "Paused");
        assert_eq!(ProtectionStatus::Disabled.as_str(), "Disabled");
    }

    #[test]
    fn test_interception_mode() {
        assert_eq!(InterceptionMode::Proxy.as_str(), "Proxy");
    }

    #[test]
    fn test_app_state_creation() {
        let db = Database::in_memory().unwrap();
        let state = AppState::new(db);

        assert!(state.is_first_setup);
        assert!(!state.is_authenticated());
        // First setup starts with Setup view
        assert_eq!(state.view, View::Setup);
    }

    #[test]
    fn test_app_state_setup_and_login() {
        let db = Database::in_memory().unwrap();
        let mut state = AppState::new(db);

        // First setup
        state.setup_password("password123").unwrap();
        assert!(!state.is_first_setup);
        assert!(state.is_authenticated());

        // Lock and re-login
        state.lock();
        assert!(!state.is_authenticated());

        state.login("password123").unwrap();
        assert!(state.is_authenticated());
    }

    #[test]
    fn test_app_state_wrong_password() {
        let db = Database::in_memory().unwrap();
        let mut state = AppState::new(db);

        state.setup_password("password123").unwrap();
        state.lock();

        let result = state.login("wrongpassword");
        assert!(result.is_err());
    }

    #[test]
    fn test_log_filter_default() {
        let filter = LogFilter::default();
        assert!(filter.profile.is_none());
        assert!(filter.action.is_none());
        assert!(filter.search.is_empty());
    }
}

//! Aegis Storage - SQLite persistence layer.
//!
//! This crate provides database storage functionality for the Aegis platform.
//! It handles:
//!
//! - Event logging (privacy-preserving: stores hashes and previews, not full prompts)
//! - Rule storage (JSON configuration)
//! - Daily statistics aggregation
//! - Configuration key-value storage
//! - Authentication (password hash storage)
//!
//! # Example
//!
//! ```no_run
//! use aegis_storage::{Database, models::{Action, NewRule}};
//! use serde_json::json;
//!
//! let db = Database::in_memory().unwrap();
//!
//! // Log an event
//! db.log_event("user prompt", None, None, Action::Allowed, None).unwrap();
//!
//! // Create a rule
//! db.create_rule(NewRule {
//!     name: "block_violence".to_string(),
//!     enabled: true,
//!     config: json!({"categories": ["violence"]}),
//!     priority: 0,
//! }).unwrap();
//! ```

mod database;
pub mod error;
pub mod models;
mod pool;
pub mod repository;
mod schema;
pub mod state_manager;

pub use database::Database;
pub use error::{Result, StorageError};
pub use models::{
    Action, Auth, CategoryCounts, Config, DailyStats, DisabledBundledSite, Event, FlaggedEvent,
    FlaggedEventFilter, FlaggedEventStats, FlaggedTypeCounts, NewEvent, NewFlaggedEvent,
    NewProfile, NewRule, NewSite, NsfwThresholdPreset, Profile, ProfileImageFilteringConfig,
    ProfileSentimentConfig, Rule, Site,
};
pub use pool::ConnectionPool;
pub use repository::{
    create_preview, create_snippet, hash_prompt, FlaggedEventsRepo, ProtectionState, SessionRecord,
    StateChange,
};
pub use state_manager::{PauseDuration, StateError, StateManager};

// Re-export for backwards compatibility
pub mod db {
    //! Database module (re-exports for compatibility).
    pub use crate::Database;
}

//! Database repositories for each table.

pub mod auth;
pub mod config;
pub mod events;
pub mod flagged;
pub mod profiles;
pub mod rules;
pub mod sites;
pub mod state;
pub mod stats;

pub use auth::AuthRepo;
pub use config::ConfigRepo;
pub use events::{create_preview, hash_prompt, EventsRepo};
pub use flagged::{create_snippet, FlaggedEventsRepo};
pub use profiles::ProfileRepo;
pub use rules::RulesRepo;
pub use sites::{DisabledBundledRepo, SiteRepo};
pub use state::{ProtectionState, SessionRecord, StateChange};
pub use stats::StatsRepo;

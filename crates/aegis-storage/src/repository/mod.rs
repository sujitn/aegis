//! Database repositories for each table.

pub mod auth;
pub mod config;
pub mod events;
pub mod profiles;
pub mod rules;
pub mod stats;

pub use auth::AuthRepo;
pub use config::ConfigRepo;
pub use events::{create_preview, hash_prompt, EventsRepo};
pub use profiles::ProfileRepo;
pub use rules::RulesRepo;
pub use stats::StatsRepo;

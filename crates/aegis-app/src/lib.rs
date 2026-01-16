//! Aegis - AI safety platform for filtering LLM interactions.
//!
//! This crate provides the main application functionality, including:
//!
//! - Clean uninstall support (F020)
//! - Autostart/persistence support (F030)
//!
//! # Usage
//!
//! ```ignore
//! use aegis_app::uninstall::{UninstallManager, UninstallOptions};
//! use aegis_app::autostart::Autostart;
//! use aegis_storage::Database;
//!
//! let db = Database::new().expect("Failed to open database");
//!
//! // Autostart
//! let autostart = Autostart::new(db.clone()).expect("Failed to create autostart");
//! autostart.enable().expect("Failed to enable autostart");
//!
//! // Uninstall
//! let mut manager = UninstallManager::new(db);
//! if manager.verify_auth("password").unwrap() {
//!     let result = manager.perform_uninstall(UninstallOptions::default());
//!     println!("Uninstall result: {:?}", result);
//! }
//! ```

pub mod autostart;
pub mod uninstall;

pub use autostart::{Autostart, AutostartError};
pub use uninstall::{UninstallManager, UninstallOptions, UninstallPaths, UninstallResult};

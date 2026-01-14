//! Aegis - AI safety platform for filtering LLM interactions.
//!
//! This crate provides the main application functionality, including:
//!
//! - Clean uninstall support (F020)
//!
//! # Usage
//!
//! ```ignore
//! use aegis_app::uninstall::{UninstallManager, UninstallOptions};
//! use aegis_storage::Database;
//!
//! let db = Database::new().expect("Failed to open database");
//! let mut manager = UninstallManager::new(db);
//!
//! // Verify authentication
//! if manager.verify_auth("password").unwrap() {
//!     let result = manager.perform_uninstall(UninstallOptions::default());
//!     println!("Uninstall result: {:?}", result);
//! }
//! ```

pub mod uninstall;

pub use uninstall::{UninstallManager, UninstallOptions, UninstallPaths, UninstallResult};

//! Clean uninstall functionality (F020).
//!
//! Provides functionality to cleanly remove all Aegis data, including:
//! - CA certificate and key files
//! - Database file
//! - Configuration files
//!
//! Requires parent authentication to prevent children from uninstalling.
//!
//! # Usage
//!
//! ```no_run
//! use aegis_app::uninstall::{UninstallManager, UninstallOptions};
//! use aegis_storage::Database;
//!
//! let db = Database::new().expect("Failed to open database");
//! let mut manager = UninstallManager::new(db);
//!
//! // First verify authentication
//! if manager.verify_auth("parent_password").unwrap() {
//!     // Perform uninstall
//!     let options = UninstallOptions::default();
//!     let result = manager.perform_uninstall(options);
//!     println!("Uninstall result: {:?}", result);
//! }
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use aegis_core::auth::AuthManager;
use aegis_storage::Database;
use directories::ProjectDirs;
use thiserror::Error;

/// Errors that can occur during uninstall.
#[derive(Debug, Error)]
pub enum UninstallError {
    /// Authentication is required.
    #[error("authentication required")]
    AuthRequired,

    /// Failed to verify password.
    #[error("failed to verify password: {0}")]
    AuthFailed(String),

    /// Failed to get data directories.
    #[error("failed to get data directories")]
    DirectoryError,

    /// Failed to delete a file or directory.
    #[error("failed to delete {path}: {reason}")]
    DeleteFailed { path: String, reason: String },

    /// Failed to export logs.
    #[error("failed to export logs: {0}")]
    ExportFailed(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Storage error.
    #[error("storage error: {0}")]
    Storage(String),
}

/// Result type for uninstall operations.
pub type Result<T> = std::result::Result<T, UninstallError>;

/// Result of an uninstall operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UninstallResult {
    /// Uninstall completed successfully.
    Success,
    /// Authentication is required before uninstall can proceed.
    AuthRequired,
    /// Uninstall partially succeeded with some errors.
    PartialSuccess {
        /// Errors encountered during uninstall.
        errors: Vec<String>,
    },
    /// Uninstall failed with an error.
    Error(String),
}

/// Options for uninstall.
#[derive(Debug, Clone, Default)]
pub struct UninstallOptions {
    /// Whether to export logs before deletion.
    pub export_logs: bool,
    /// Path for log export (if export_logs is true).
    pub export_path: Option<PathBuf>,
}

/// Paths that will be deleted during uninstall.
#[derive(Debug, Clone)]
pub struct UninstallPaths {
    /// The main data directory.
    pub data_dir: PathBuf,
    /// The CA certificate directory.
    pub ca_dir: PathBuf,
    /// The database file path.
    pub database: PathBuf,
}

impl UninstallPaths {
    /// Get paths for the default Aegis installation.
    pub fn default_paths() -> Option<Self> {
        // Storage uses "com.aegis.aegis"
        let storage_dirs = ProjectDirs::from("com", "aegis", "aegis")?;
        // Proxy uses "com.aegis.Aegis" (note capital A)
        let proxy_dirs = ProjectDirs::from("com", "aegis", "Aegis")?;

        Some(Self {
            data_dir: storage_dirs.data_dir().to_path_buf(),
            ca_dir: proxy_dirs.data_dir().join("ca"),
            database: storage_dirs.data_dir().join("aegis.db"),
        })
    }
}

/// Manages clean uninstall operations.
pub struct UninstallManager {
    db: Database,
    auth: AuthManager,
    authenticated: bool,
}

impl UninstallManager {
    /// Create a new uninstall manager.
    pub fn new(db: Database) -> Self {
        Self {
            db,
            auth: AuthManager::new(),
            authenticated: false,
        }
    }

    /// Verify parent authentication before uninstall.
    ///
    /// Returns `true` if authentication succeeds.
    pub fn verify_auth(&mut self, password: &str) -> Result<bool> {
        // Get stored password hash
        let hash = self
            .db
            .get_password_hash()
            .map_err(|e| UninstallError::AuthFailed(e.to_string()))?;

        // Verify password
        let verified = self
            .auth
            .verify_password(password, &hash)
            .map_err(|e| UninstallError::AuthFailed(e.to_string()))?;

        if verified {
            self.authenticated = true;
        }

        Ok(verified)
    }

    /// Check if authentication has been verified.
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Get paths that will be deleted.
    pub fn get_data_paths() -> Option<UninstallPaths> {
        UninstallPaths::default_paths()
    }

    /// Get OS-specific CA removal instructions.
    #[cfg(target_os = "windows")]
    pub fn get_ca_removal_instructions() -> &'static str {
        r#"To remove the Aegis CA certificate from Windows:

1. Open Command Prompt as Administrator
2. Run: certutil -delstore Root "Aegis Root CA"

Or manually via Certificate Manager:
1. Press Win+R, type "certmgr.msc", press Enter
2. Navigate to Trusted Root Certification Authorities > Certificates
3. Find "Aegis Root CA", right-click and select Delete"#
    }

    /// Get OS-specific CA removal instructions.
    #[cfg(target_os = "macos")]
    pub fn get_ca_removal_instructions() -> &'static str {
        r#"To remove the Aegis CA certificate from macOS:

Terminal:
  sudo security delete-certificate -c "Aegis Root CA" /Library/Keychains/System.keychain

Or manually via Keychain Access:
1. Open Keychain Access (Applications > Utilities)
2. Select "System" keychain
3. Find "Aegis Root CA" in Certificates
4. Right-click and select "Delete""#
    }

    /// Get OS-specific CA removal instructions.
    #[cfg(target_os = "linux")]
    pub fn get_ca_removal_instructions() -> &'static str {
        r#"To remove the Aegis CA certificate from Linux:

Ubuntu/Debian:
  sudo rm /usr/local/share/ca-certificates/aegis-ca.crt
  sudo update-ca-certificates --fresh

Fedora/RHEL:
  sudo rm /etc/pki/ca-trust/source/anchors/aegis-ca.crt
  sudo update-ca-trust

Arch Linux:
  sudo rm /etc/ca-certificates/trust-source/anchors/aegis-ca.crt
  sudo trust extract-compat"#
    }

    /// Get OS-specific CA removal instructions (fallback).
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn get_ca_removal_instructions() -> &'static str {
        r#"To remove the Aegis CA certificate:

Please consult your operating system's documentation for removing
trusted root certificates. Look for a certificate named "Aegis Root CA"
in your system's certificate store."#
    }

    /// Export logs to CSV before uninstall.
    pub fn export_logs(&self, path: &Path) -> Result<usize> {
        let events = self
            .db
            .get_recent_events(10000, 0)
            .map_err(|e| UninstallError::ExportFailed(e.to_string()))?;

        let file = fs::File::create(path)?;
        let mut writer = csv::Writer::from_writer(file);

        // Write header
        writer
            .write_record([
                "ID",
                "Created At",
                "Preview",
                "Category",
                "Action",
                "Source",
            ])
            .map_err(|e| UninstallError::ExportFailed(e.to_string()))?;

        // Write events
        for event in &events {
            let category = event
                .category
                .map(|c| format!("{:?}", c))
                .unwrap_or_default();

            writer
                .write_record([
                    &event.id.to_string(),
                    &event.created_at.to_string(),
                    &event.preview,
                    &category,
                    &format!("{:?}", event.action),
                    event.source.as_deref().unwrap_or(""),
                ])
                .map_err(|e| UninstallError::ExportFailed(e.to_string()))?;
        }

        writer
            .flush()
            .map_err(|e| UninstallError::ExportFailed(e.to_string()))?;

        Ok(events.len())
    }

    /// Perform clean uninstall (requires prior auth verification).
    pub fn perform_uninstall(&self, options: UninstallOptions) -> UninstallResult {
        // Check authentication
        if !self.authenticated {
            return UninstallResult::AuthRequired;
        }

        let paths = match Self::get_data_paths() {
            Some(p) => p,
            None => return UninstallResult::Error("Failed to get data paths".to_string()),
        };

        let mut errors = Vec::new();

        // Export logs if requested
        if options.export_logs {
            let export_path = options.export_path.unwrap_or_else(|| {
                let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                PathBuf::from(format!("aegis_logs_export_{}.csv", timestamp))
            });

            if let Err(e) = self.export_logs(&export_path) {
                errors.push(format!("Failed to export logs: {}", e));
            }
        }

        // Delete CA certificate files
        if paths.ca_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&paths.ca_dir) {
                errors.push(format!(
                    "Failed to delete CA directory {}: {}",
                    paths.ca_dir.display(),
                    e
                ));
            }
        }

        // Delete database file
        if paths.database.exists() {
            if let Err(e) = fs::remove_file(&paths.database) {
                errors.push(format!(
                    "Failed to delete database {}: {}",
                    paths.database.display(),
                    e
                ));
            }
        }

        // Delete entire data directory (includes config)
        if paths.data_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&paths.data_dir) {
                errors.push(format!(
                    "Failed to delete data directory {}: {}",
                    paths.data_dir.display(),
                    e
                ));
            }
        }

        // Return result based on errors
        if errors.is_empty() {
            UninstallResult::Success
        } else {
            UninstallResult::PartialSuccess { errors }
        }
    }

    /// Delete a specific path (file or directory).
    pub fn delete_path(path: &Path) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        if path.is_dir() {
            fs::remove_dir_all(path).map_err(|e| UninstallError::DeleteFailed {
                path: path.display().to_string(),
                reason: e.to_string(),
            })
        } else {
            fs::remove_file(path).map_err(|e| UninstallError::DeleteFailed {
                path: path.display().to_string(),
                reason: e.to_string(),
            })
        }
    }
}

/// Generate uninstall confirmation text.
pub fn get_confirmation_text(paths: &UninstallPaths) -> String {
    format!(
        r#"This will permanently delete all Aegis data:

  - Database: {}
  - CA certificates: {}
  - Configuration: {}

This action cannot be undone. Are you sure you want to continue?"#,
        paths.database.display(),
        paths.ca_dir.display(),
        paths.data_dir.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_db() -> Database {
        Database::in_memory().expect("Failed to create test database")
    }

    fn setup_test_db_with_auth() -> Database {
        let db = create_test_db();
        let auth = AuthManager::new();
        let hash = auth.hash_password("test_password").unwrap();
        db.set_password_hash(&hash).unwrap();
        db
    }

    // ==================== UninstallPaths Tests ====================

    #[test]
    fn test_uninstall_paths_default() {
        // Should return Some on most systems
        let paths = UninstallPaths::default_paths();
        // May be None in some CI environments, so just test it doesn't panic
        if let Some(p) = paths {
            assert!(!p.data_dir.as_os_str().is_empty());
            assert!(!p.ca_dir.as_os_str().is_empty());
            assert!(!p.database.as_os_str().is_empty());
        }
    }

    // ==================== UninstallManager Tests ====================

    #[test]
    fn test_uninstall_manager_new() {
        let db = create_test_db();
        let manager = UninstallManager::new(db);
        assert!(!manager.is_authenticated());
    }

    #[test]
    fn test_verify_auth_success() {
        let db = setup_test_db_with_auth();
        let mut manager = UninstallManager::new(db);

        let result = manager.verify_auth("test_password").unwrap();
        assert!(result);
        assert!(manager.is_authenticated());
    }

    #[test]
    fn test_verify_auth_wrong_password() {
        let db = setup_test_db_with_auth();
        let mut manager = UninstallManager::new(db);

        let result = manager.verify_auth("wrong_password").unwrap();
        assert!(!result);
        assert!(!manager.is_authenticated());
    }

    #[test]
    fn test_uninstall_requires_auth() {
        let db = create_test_db();
        let manager = UninstallManager::new(db);

        let result = manager.perform_uninstall(UninstallOptions::default());
        assert_eq!(result, UninstallResult::AuthRequired);
    }

    #[test]
    fn test_get_ca_removal_instructions() {
        let instructions = UninstallManager::get_ca_removal_instructions();
        assert!(!instructions.is_empty());
        assert!(instructions.contains("Aegis"));
    }

    // ==================== Export Tests ====================

    #[test]
    fn test_export_logs_empty_db() {
        let db = create_test_db();
        let manager = UninstallManager::new(db);

        let temp_dir = TempDir::new().unwrap();
        let export_path = temp_dir.path().join("logs.csv");

        let count = manager.export_logs(&export_path).unwrap();
        assert_eq!(count, 0);
        assert!(export_path.exists());
    }

    #[test]
    fn test_export_logs_with_events() {
        let db = create_test_db();

        // Log some events
        db.log_event(
            "test prompt 1",
            Some(aegis_core::classifier::Category::Violence),
            Some(0.9),
            aegis_storage::models::Action::Blocked,
            Some("test".to_string()),
        )
        .unwrap();
        db.log_event(
            "test prompt 2",
            None,
            None,
            aegis_storage::models::Action::Allowed,
            None,
        )
        .unwrap();

        let manager = UninstallManager::new(db);

        let temp_dir = TempDir::new().unwrap();
        let export_path = temp_dir.path().join("logs.csv");

        let count = manager.export_logs(&export_path).unwrap();
        assert_eq!(count, 2);

        // Verify CSV content
        let content = fs::read_to_string(&export_path).unwrap();
        assert!(content.contains("ID,Created At,Preview,Category,Action,Source"));
        assert!(content.contains("test prompt 1"));
        assert!(content.contains("Violence"));
    }

    // ==================== Delete Path Tests ====================

    #[test]
    fn test_delete_path_nonexistent() {
        let result = UninstallManager::delete_path(Path::new("/nonexistent/path"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_path_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "test").unwrap();
        assert!(file_path.exists());

        UninstallManager::delete_path(&file_path).unwrap();
        assert!(!file_path.exists());
    }

    #[test]
    fn test_delete_path_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("subdir");

        fs::create_dir(&dir_path).unwrap();
        fs::write(dir_path.join("file.txt"), "test").unwrap();
        assert!(dir_path.exists());

        UninstallManager::delete_path(&dir_path).unwrap();
        assert!(!dir_path.exists());
    }

    // ==================== UninstallOptions Tests ====================

    #[test]
    fn test_uninstall_options_default() {
        let options = UninstallOptions::default();
        assert!(!options.export_logs);
        assert!(options.export_path.is_none());
    }

    // ==================== UninstallResult Tests ====================

    #[test]
    fn test_uninstall_result_equality() {
        assert_eq!(UninstallResult::Success, UninstallResult::Success);
        assert_eq!(UninstallResult::AuthRequired, UninstallResult::AuthRequired);
        assert_eq!(
            UninstallResult::Error("test".to_string()),
            UninstallResult::Error("test".to_string())
        );
        assert_ne!(UninstallResult::Success, UninstallResult::AuthRequired);
    }

    // ==================== Confirmation Text Tests ====================

    #[test]
    fn test_get_confirmation_text() {
        let paths = UninstallPaths {
            data_dir: PathBuf::from("/data"),
            ca_dir: PathBuf::from("/data/ca"),
            database: PathBuf::from("/data/aegis.db"),
        };

        let text = get_confirmation_text(&paths);
        assert!(text.contains("/data"));
        assert!(text.contains("/data/ca"));
        assert!(text.contains("/data/aegis.db"));
        assert!(text.contains("permanently delete"));
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_full_uninstall_flow() {
        let temp_dir = TempDir::new().unwrap();

        // Create test data structure
        let data_dir = temp_dir.path().join("data");
        let ca_dir = temp_dir.path().join("ca");
        fs::create_dir_all(&data_dir).unwrap();
        fs::create_dir_all(&ca_dir).unwrap();

        // Create test files
        fs::write(data_dir.join("aegis.db"), "test database").unwrap();
        fs::write(data_dir.join("config.json"), "{}").unwrap();
        fs::write(ca_dir.join("aegis-ca.crt"), "cert").unwrap();
        fs::write(ca_dir.join("aegis-ca.key"), "key").unwrap();

        // Verify files exist
        assert!(data_dir.exists());
        assert!(ca_dir.exists());

        // Delete the paths
        UninstallManager::delete_path(&ca_dir).unwrap();
        UninstallManager::delete_path(&data_dir).unwrap();

        // Verify deletion
        assert!(!data_dir.exists());
        assert!(!ca_dir.exists());
    }

    #[test]
    fn test_uninstall_with_auth_and_export() {
        let db = setup_test_db_with_auth();

        // Log an event
        db.log_event(
            "test prompt",
            None,
            None,
            aegis_storage::models::Action::Allowed,
            None,
        )
        .unwrap();

        let mut manager = UninstallManager::new(db);

        // Authenticate
        assert!(manager.verify_auth("test_password").unwrap());

        // Create temp export location
        let temp_dir = TempDir::new().unwrap();
        let export_path = temp_dir.path().join("export.csv");

        // Export logs
        let count = manager.export_logs(&export_path).unwrap();
        assert_eq!(count, 1);
        assert!(export_path.exists());
    }
}

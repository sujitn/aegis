# F020: Clean Uninstall

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | high | aegis-app |

## Description

Clean uninstall functionality that removes all Aegis data and configuration. Requires parent authentication to prevent children from uninstalling protection. Provides OS-specific instructions for removing the CA certificate from the system trust store (if proxy mode was used).

## Dependencies

- **Requires**: F013 (Authentication), F016 (MITM Proxy)
- **Blocks**: None

## Acceptance Criteria

- [x] Require parent password before uninstall proceeds
- [x] Delete CA certificate and key files from data directory
- [x] Delete database file (aegis.db)
- [x] Delete config directory contents
- [x] Provide OS-specific CA removal instructions (Windows/macOS/Linux)
- [x] Option to export logs before deletion
- [x] Confirmation dialog before destructive operations
- [x] UninstallManager type with clean API

## API

```rust
/// Result of an uninstall operation.
pub enum UninstallResult {
    Success,
    AuthRequired,
    PartialSuccess { errors: Vec<String> },
    Error(String),
}

/// Options for uninstall.
pub struct UninstallOptions {
    /// Whether to export logs before deletion.
    pub export_logs: bool,
    /// Path for log export (if export_logs is true).
    pub export_path: Option<PathBuf>,
}

/// Manages clean uninstall operations.
pub struct UninstallManager {
    db: Database,
}

impl UninstallManager {
    /// Verify parent authentication before uninstall.
    pub fn verify_auth(&self, password: &str) -> Result<bool>;

    /// Get paths that will be deleted.
    pub fn get_data_paths() -> UninstallPaths;

    /// Get OS-specific CA removal instructions.
    pub fn get_ca_removal_instructions() -> &'static str;

    /// Perform clean uninstall (requires prior auth verification).
    pub fn perform_uninstall(&self, options: UninstallOptions) -> UninstallResult;

    /// Export logs to CSV before uninstall.
    pub fn export_logs(&self, path: &Path) -> Result<()>;
}

/// Paths that will be deleted during uninstall.
pub struct UninstallPaths {
    pub data_dir: PathBuf,
    pub ca_dir: PathBuf,
    pub database: PathBuf,
}
```

## OS-Specific CA Removal

### Windows
```
certutil -delstore Root "Aegis Root CA"
```

### macOS
```
sudo security delete-certificate -c "Aegis Root CA" /Library/Keychains/System.keychain
```

### Linux
```
sudo rm /usr/local/share/ca-certificates/aegis-ca.crt
sudo update-ca-certificates --fresh
```

## Notes

- Authentication required to prevent children from disabling protection
- CA removal from system trust store may require elevated privileges
- Provide instructions rather than automatic removal for system store
- Log export uses same CSV format as dashboard export
- Partial success if some files can't be deleted (permissions)

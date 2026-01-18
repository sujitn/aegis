//! Browser extension auto-installation support.
//!
//! Provides platform-specific methods to install the Aegis browser extension
//! using Chrome's enterprise policy mechanisms.
//!
//! # Features
//!
//! This module is only compiled when the `extension-install` feature is enabled.
//!
//! # Platform Support
//!
//! - **Windows**: Uses registry-based external extensions
//! - **macOS**: Uses external extensions JSON in Application Support
//! - **Linux**: Uses external extensions JSON in ~/.config

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Result of an extension installation attempt.
#[derive(Debug, Clone)]
pub struct ExtensionInstallResult {
    /// Whether the installation was successful.
    pub success: bool,
    /// Human-readable message about the result.
    pub message: String,
    /// Whether admin/root privileges are required.
    pub needs_admin: bool,
}

impl ExtensionInstallResult {
    /// Creates a successful result.
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            needs_admin: false,
        }
    }

    /// Creates an error result.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            needs_admin: false,
        }
    }

    /// Creates a result indicating admin privileges are needed.
    pub fn needs_admin(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            needs_admin: true,
        }
    }
}

/// Errors that can occur during extension installation.
#[derive(Debug, Error)]
pub enum ExtensionInstallError {
    #[error("Extension path does not exist: {0}")]
    PathNotFound(PathBuf),

    #[error("Invalid extension: manifest.json not found")]
    InvalidExtension,

    #[error("Registry error: {0}")]
    RegistryError(String),

    #[error("File system error: {0}")]
    FileSystemError(#[from] std::io::Error),

    #[error("Requires administrator privileges")]
    NeedsAdmin,

    #[error("Unsupported platform")]
    UnsupportedPlatform,
}

/// Browser type for extension installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Browser {
    Chrome,
    Edge,
    Brave,
    Chromium,
}

impl Browser {
    /// Returns all supported browsers.
    pub fn all() -> &'static [Browser] {
        &[
            Browser::Chrome,
            Browser::Edge,
            Browser::Brave,
            Browser::Chromium,
        ]
    }

    /// Returns the display name.
    pub fn name(&self) -> &'static str {
        match self {
            Browser::Chrome => "Google Chrome",
            Browser::Edge => "Microsoft Edge",
            Browser::Brave => "Brave",
            Browser::Chromium => "Chromium",
        }
    }
}

/// Installs the extension for all supported browsers.
///
/// This attempts to install using the external extensions JSON method which
/// works without admin rights for the current user only.
pub fn install_extension(extension_path: &Path) -> ExtensionInstallResult {
    // Validate extension path
    if !extension_path.exists() {
        return ExtensionInstallResult::error(format!(
            "Extension path does not exist: {}",
            extension_path.display()
        ));
    }

    if !extension_path.join("manifest.json").exists() {
        return ExtensionInstallResult::error("Invalid extension: manifest.json not found");
    }

    // Try to install for each browser
    let mut successes = Vec::new();
    let mut errors = Vec::new();

    for browser in Browser::all() {
        match install_for_browser(extension_path, *browser) {
            Ok(()) => successes.push(browser.name()),
            Err(e) => errors.push(format!("{}: {}", browser.name(), e)),
        }
    }

    if successes.is_empty() {
        ExtensionInstallResult::error(format!(
            "Failed to install for any browser. Errors: {}",
            errors.join("; ")
        ))
    } else {
        ExtensionInstallResult::success(format!(
            "Extension registered for: {}. Restart browser and check chrome://extensions to enable.",
            successes.join(", ")
        ))
    }
}

/// Installs the extension for a specific browser.
fn install_for_browser(
    extension_path: &Path,
    browser: Browser,
) -> Result<(), ExtensionInstallError> {
    #[cfg(target_os = "windows")]
    {
        install_windows(extension_path, browser)
    }

    #[cfg(target_os = "macos")]
    {
        install_macos(extension_path, browser)
    }

    #[cfg(target_os = "linux")]
    {
        install_linux(extension_path, browser)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = (extension_path, browser);
        Err(ExtensionInstallError::UnsupportedPlatform)
    }
}

/// Uninstalls the extension from all browsers.
pub fn uninstall_extension() -> ExtensionInstallResult {
    let mut successes = Vec::new();
    let mut errors = Vec::new();

    for browser in Browser::all() {
        match uninstall_for_browser(*browser) {
            Ok(()) => successes.push(browser.name()),
            Err(e) => errors.push(format!("{}: {}", browser.name(), e)),
        }
    }

    if successes.is_empty() && errors.is_empty() {
        ExtensionInstallResult::success("No extension installations found to remove")
    } else if !successes.is_empty() {
        ExtensionInstallResult::success(format!(
            "Extension removed from: {}. Restart browser(s) to apply.",
            successes.join(", ")
        ))
    } else {
        ExtensionInstallResult::error(format!("Failed to uninstall: {}", errors.join("; ")))
    }
}

/// Uninstalls the extension from a specific browser.
fn uninstall_for_browser(browser: Browser) -> Result<(), ExtensionInstallError> {
    #[cfg(target_os = "windows")]
    {
        uninstall_windows(browser)
    }

    #[cfg(target_os = "macos")]
    {
        uninstall_macos(browser)
    }

    #[cfg(target_os = "linux")]
    {
        uninstall_linux(browser)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = browser;
        Err(ExtensionInstallError::UnsupportedPlatform)
    }
}

// =============================================================================
// Windows Implementation
// =============================================================================

#[cfg(target_os = "windows")]
fn install_windows(extension_path: &Path, browser: Browser) -> Result<(), ExtensionInstallError> {
    use winreg::enums::*;
    use winreg::RegKey;

    // Read the manifest to get the version
    let manifest_path = extension_path.join("manifest.json");
    let manifest_content = std::fs::read_to_string(&manifest_path)?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content).map_err(|e| {
        ExtensionInstallError::FileSystemError(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            e.to_string(),
        ))
    })?;
    let version = manifest["version"].as_str().unwrap_or("1.0.0");

    // Look for CRX file in release folder
    let release_dir = extension_path.join("release");
    let crx_path = if release_dir.exists() {
        // Find the CRX file
        std::fs::read_dir(&release_dir).ok().and_then(|entries| {
            entries
                .filter_map(|e| e.ok())
                .find(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "crx")
                        .unwrap_or(false)
                })
                .map(|e| e.path())
        })
    } else {
        None
    };

    // Get the registry path for external extensions
    let reg_path = get_windows_external_extensions_path(browser);

    // Open or create the registry key
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(&reg_path)
        .map_err(|e| ExtensionInstallError::RegistryError(e.to_string()))?;

    // Read extension ID from manifest, or use default
    let extension_id = manifest["key"]
        .as_str()
        .map(|_| "aegis-extension") // If there's a key, we'll use our ID
        .unwrap_or("aegis-extension");

    // Create the extension entry JSON
    // Use external_crx if CRX file exists, otherwise fall back to update URL
    let extension_json = if let Some(ref crx) = crx_path {
        // Use CRX file directly - this is the most reliable method
        serde_json::json!({
            "external_crx": crx.display().to_string().replace('\\', "/"),
            "external_version": version
        })
    } else {
        // Fall back to update URL method (less reliable for local files)
        create_updates_xml(extension_path)?;
        serde_json::json!({
            "external_update_url": format!("file:///{}/updates.xml", extension_path.display().to_string().replace('\\', "/"))
        })
    };

    // Set the registry value
    key.set_value(extension_id, &extension_json.to_string())
        .map_err(|e| ExtensionInstallError::RegistryError(e.to_string()))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn uninstall_windows(browser: Browser) -> Result<(), ExtensionInstallError> {
    use winreg::enums::*;
    use winreg::RegKey;

    let reg_path = get_windows_external_extensions_path(browser);
    let extension_id = "aegis-extension";

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    if let Ok(key) = hkcu.open_subkey_with_flags(&reg_path, KEY_WRITE) {
        let _ = key.delete_value(extension_id);
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn get_windows_external_extensions_path(browser: Browser) -> String {
    match browser {
        Browser::Chrome => r"Software\Google\Chrome\Extensions".to_string(),
        Browser::Edge => r"Software\Microsoft\Edge\Extensions".to_string(),
        Browser::Brave => r"Software\BraveSoftware\Brave-Browser\Extensions".to_string(),
        Browser::Chromium => r"Software\Chromium\Extensions".to_string(),
    }
}

// =============================================================================
// macOS Implementation
// =============================================================================

#[cfg(target_os = "macos")]
fn install_macos(extension_path: &Path, browser: Browser) -> Result<(), ExtensionInstallError> {
    // macOS uses external extensions JSON in the browser's Application Support directory
    let ext_dir = get_macos_external_extensions_dir(browser)?;
    std::fs::create_dir_all(&ext_dir)?;

    let extension_id = "aegis-extension";
    let json_path = ext_dir.join(format!("{}.json", extension_id));

    // For local unpacked extensions, point to update manifest
    let json_content = serde_json::json!({
        "external_update_url": format!("file://{}/updates.xml", extension_path.display())
    });

    std::fs::write(
        &json_path,
        serde_json::to_string_pretty(&json_content).unwrap_or_default(),
    )?;

    // Create updates.xml
    create_updates_xml(extension_path)?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn uninstall_macos(browser: Browser) -> Result<(), ExtensionInstallError> {
    if let Ok(ext_dir) = get_macos_external_extensions_dir(browser) {
        let extension_id = "aegis-extension";
        let json_path = ext_dir.join(format!("{}.json", extension_id));
        let _ = std::fs::remove_file(json_path);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn get_macos_external_extensions_dir(browser: Browser) -> Result<PathBuf, ExtensionInstallError> {
    let home = std::env::var("HOME").map_err(|_| {
        ExtensionInstallError::FileSystemError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "HOME not set",
        ))
    })?;

    let path = match browser {
        Browser::Chrome => PathBuf::from(&home)
            .join("Library/Application Support/Google/Chrome/External Extensions"),
        Browser::Edge => PathBuf::from(&home)
            .join("Library/Application Support/Microsoft Edge/External Extensions"),
        Browser::Brave => PathBuf::from(&home)
            .join("Library/Application Support/BraveSoftware/Brave-Browser/External Extensions"),
        Browser::Chromium => {
            PathBuf::from(&home).join("Library/Application Support/Chromium/External Extensions")
        }
    };

    Ok(path)
}

// =============================================================================
// Linux Implementation
// =============================================================================

#[cfg(target_os = "linux")]
fn install_linux(extension_path: &Path, browser: Browser) -> Result<(), ExtensionInstallError> {
    // Linux uses external extensions JSON in ~/.config/<browser>/External Extensions/
    let ext_dir = get_linux_external_extensions_dir(browser)?;
    std::fs::create_dir_all(&ext_dir)?;

    let extension_id = "aegis-extension";
    let json_path = ext_dir.join(format!("{}.json", extension_id));

    let json_content = serde_json::json!({
        "external_update_url": format!("file://{}/updates.xml", extension_path.display())
    });

    std::fs::write(
        &json_path,
        serde_json::to_string_pretty(&json_content).unwrap_or_default(),
    )?;

    // Create updates.xml
    create_updates_xml(extension_path)?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn uninstall_linux(browser: Browser) -> Result<(), ExtensionInstallError> {
    if let Ok(ext_dir) = get_linux_external_extensions_dir(browser) {
        let extension_id = "aegis-extension";
        let json_path = ext_dir.join(format!("{}.json", extension_id));
        let _ = std::fs::remove_file(json_path);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn get_linux_external_extensions_dir(browser: Browser) -> Result<PathBuf, ExtensionInstallError> {
    let home = std::env::var("HOME").map_err(|_| {
        ExtensionInstallError::FileSystemError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "HOME not set",
        ))
    })?;

    let path = match browser {
        Browser::Chrome => PathBuf::from(&home).join(".config/google-chrome/External Extensions"),
        Browser::Edge => PathBuf::from(&home).join(".config/microsoft-edge/External Extensions"),
        Browser::Brave => {
            PathBuf::from(&home).join(".config/BraveSoftware/Brave-Browser/External Extensions")
        }
        Browser::Chromium => PathBuf::from(&home).join(".config/chromium/External Extensions"),
    };

    Ok(path)
}

// =============================================================================
// Shared Helpers
// =============================================================================

/// Creates an updates.xml file for Chrome's external extension update mechanism.
fn create_updates_xml(extension_path: &Path) -> Result<(), ExtensionInstallError> {
    // Read the manifest to get the version
    let manifest_path = extension_path.join("manifest.json");
    let manifest_content = std::fs::read_to_string(&manifest_path)?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content).map_err(|e| {
        ExtensionInstallError::FileSystemError(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            e.to_string(),
        ))
    })?;

    let version = manifest["version"].as_str().unwrap_or("1.0.0");

    // Create updates.xml
    // Note: For unpacked extensions, Chrome actually doesn't use this file,
    // but we create it for completeness
    let updates_xml = format!(
        r#"<?xml version='1.0' encoding='UTF-8'?>
<gupdate xmlns='http://www.google.com/update2/response' protocol='2.0'>
  <app appid='aegis-extension'>
    <updatecheck codebase='file://{}' version='{}'/>
  </app>
</gupdate>"#,
        extension_path.display().to_string().replace('\\', "/"),
        version
    );

    let updates_path = extension_path.join("updates.xml");
    std::fs::write(&updates_path, updates_xml)?;

    Ok(())
}

/// Returns the path to the bundled extension folder.
pub fn get_extension_path() -> Option<PathBuf> {
    // Try to find extension relative to executable
    if let Ok(exe_path) = std::env::current_exe() {
        let exe_dir = exe_path.parent()?;

        // Check common locations for installed app
        let candidates = [
            // Windows MSI installs to: Program Files/Aegis/bin/aegis.exe
            // Extension at: Program Files/Aegis/extension/
            exe_dir.join("..").join("extension"),
            // Same directory as executable
            exe_dir.join("extension"),
            // Resources subfolder
            exe_dir.join("resources").join("extension"),
            // macOS bundle: Aegis.app/Contents/MacOS/aegis -> ../Resources/extension
            exe_dir.join("..").join("Resources").join("extension"),
        ];

        for candidate in &candidates {
            if candidate.join("manifest.json").exists() {
                // Canonicalize to clean up the path
                if let Ok(canonical) = candidate.canonicalize() {
                    return Some(canonical);
                }
                return Some(candidate.clone());
            }
        }

        // Linux system-wide install locations
        #[cfg(target_os = "linux")]
        {
            let linux_candidates = [
                // DEB/RPM install location
                PathBuf::from("/usr/share/aegis/extension"),
                // AppImage: extracted to /tmp, look relative to usr/bin
                exe_dir
                    .join("..")
                    .join("share")
                    .join("aegis")
                    .join("extension"),
            ];
            for candidate in &linux_candidates {
                if candidate.join("manifest.json").exists() {
                    if let Ok(canonical) = candidate.canonicalize() {
                        return Some(canonical);
                    }
                    return Some(candidate.clone());
                }
            }
        }

        // Fallback: development path (from target/debug or target/release)
        #[cfg(debug_assertions)]
        {
            let dev_candidates = [
                // target/debug -> target -> project root
                exe_dir.join("..").join("..").join("extension"),
                // target/debug/deps -> target/debug -> target -> project root
                exe_dir.join("..").join("..").join("..").join("extension"),
            ];
            for candidate in &dev_candidates {
                if candidate.join("manifest.json").exists() {
                    if let Ok(canonical) = candidate.canonicalize() {
                        return Some(canonical);
                    }
                    return Some(candidate.clone());
                }
            }
        }
    }

    None
}

/// Checks if the extension appears to be installed for any browser.
pub fn is_extension_installed() -> bool {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        for browser in Browser::all() {
            let reg_path = get_windows_external_extensions_path(*browser);
            if let Ok(key) = hkcu.open_subkey(&reg_path) {
                if key.get_value::<String, _>("aegis-extension").is_ok() {
                    return true;
                }
            }
        }
        false
    }

    #[cfg(target_os = "macos")]
    {
        for browser in Browser::all() {
            if let Ok(ext_dir) = get_macos_external_extensions_dir(*browser) {
                let json_path = ext_dir.join("aegis-extension.json");
                if json_path.exists() {
                    return true;
                }
            }
        }
        false
    }

    #[cfg(target_os = "linux")]
    {
        for browser in Browser::all() {
            if let Ok(ext_dir) = get_linux_external_extensions_dir(*browser) {
                let json_path = ext_dir.join("aegis-extension.json");
                if json_path.exists() {
                    return true;
                }
            }
        }
        false
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_install_result() {
        let success = ExtensionInstallResult::success("Test");
        assert!(success.success);
        assert!(!success.needs_admin);

        let error = ExtensionInstallResult::error("Failed");
        assert!(!error.success);
        assert!(!error.needs_admin);

        let admin = ExtensionInstallResult::needs_admin("Need admin");
        assert!(!admin.success);
        assert!(admin.needs_admin);
    }

    #[test]
    fn test_browser_all() {
        let browsers = Browser::all();
        assert!(browsers.len() >= 2);
        assert!(browsers.contains(&Browser::Chrome));
        assert!(browsers.contains(&Browser::Edge));
    }

    #[test]
    fn test_get_extension_path() {
        // This will likely return None in test environment
        // Just ensure it doesn't panic
        let _ = get_extension_path();
    }
}

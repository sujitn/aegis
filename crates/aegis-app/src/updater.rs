//! Auto-update functionality for Aegis.
//!
//! Checks GitHub Releases for new versions and handles downloading updates.

use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Default GitHub repository for updates.
pub const DEFAULT_REPO_OWNER: &str = "aegis";
pub const DEFAULT_REPO_NAME: &str = "aegis";

/// Update check interval in seconds (24 hours).
pub const DEFAULT_CHECK_INTERVAL_SECS: u64 = 86400;

/// Errors that can occur during update operations.
#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Failed to parse version: {0}")]
    VersionParse(#[from] semver::Error),

    #[error("Failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("No compatible asset found for this platform")]
    NoCompatibleAsset,

    #[error("Download cancelled")]
    Cancelled,

    #[error("Update check disabled")]
    Disabled,
}

/// Result type for update operations.
pub type Result<T> = std::result::Result<T, UpdateError>;

/// A GitHub release.
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub html_url: String,
    pub published_at: Option<String>,
    pub prerelease: bool,
    pub draft: bool,
    pub assets: Vec<GitHubAsset>,
}

/// A release asset (downloadable file).
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    pub content_type: String,
}

/// Information about an available update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    /// New version available.
    pub version: String,
    /// Release name/title.
    pub name: Option<String>,
    /// Changelog/release notes.
    pub changelog: Option<String>,
    /// URL to the release page.
    pub release_url: String,
    /// Download URL for this platform.
    pub download_url: Option<String>,
    /// Download size in bytes.
    pub download_size: Option<u64>,
    /// Asset filename.
    pub asset_name: Option<String>,
    /// When the release was published.
    pub published_at: Option<String>,
    /// Whether this is a pre-release.
    pub prerelease: bool,
}

/// Update check settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettings {
    /// Whether automatic update checks are enabled.
    pub enabled: bool,
    /// Whether to include pre-release versions.
    pub include_prereleases: bool,
    /// Whether to automatically download updates.
    pub auto_download: bool,
    /// Check interval in seconds.
    pub check_interval_secs: u64,
    /// GitHub repository owner.
    pub repo_owner: String,
    /// GitHub repository name.
    pub repo_name: String,
    /// Last check timestamp (ISO 8601).
    pub last_check: Option<String>,
    /// Last known version that was dismissed.
    pub dismissed_version: Option<String>,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            include_prereleases: false,
            auto_download: false,
            check_interval_secs: DEFAULT_CHECK_INTERVAL_SECS,
            repo_owner: DEFAULT_REPO_OWNER.to_string(),
            repo_name: DEFAULT_REPO_NAME.to_string(),
            last_check: None,
            dismissed_version: None,
        }
    }
}

/// Download progress information.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Bytes downloaded so far.
    pub downloaded: u64,
    /// Total bytes to download.
    pub total: u64,
    /// Download speed in bytes per second.
    pub speed: u64,
}

impl DownloadProgress {
    /// Returns the download progress as a percentage (0.0 - 100.0).
    pub fn percent(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.downloaded as f64 / self.total as f64) * 100.0
        }
    }
}

/// State of an ongoing or completed update.
#[derive(Debug, Clone)]
pub enum UpdateState {
    /// No update activity.
    Idle,
    /// Checking for updates.
    Checking,
    /// Update available.
    Available(UpdateInfo),
    /// Downloading update.
    Downloading(DownloadProgress),
    /// Download complete, ready to install.
    Ready(PathBuf),
    /// Error occurred.
    Error(String),
}

/// Manages automatic updates.
#[derive(Debug)]
pub struct UpdateManager {
    settings: RwLock<UpdateSettings>,
    state: RwLock<UpdateState>,
    current_version: Version,
    client: reqwest::Client,
    download_dir: PathBuf,
}

impl UpdateManager {
    /// Creates a new UpdateManager with default settings.
    pub fn new(download_dir: PathBuf) -> Result<Self> {
        Self::with_settings(download_dir, UpdateSettings::default())
    }

    /// Creates a new UpdateManager with custom settings.
    pub fn with_settings(download_dir: PathBuf, settings: UpdateSettings) -> Result<Self> {
        let current_version = Version::parse(env!("CARGO_PKG_VERSION"))?;

        let client = reqwest::Client::builder()
            .user_agent(format!("Aegis/{}", env!("CARGO_PKG_VERSION")))
            .build()?;

        Ok(Self {
            settings: RwLock::new(settings),
            state: RwLock::new(UpdateState::Idle),
            current_version,
            client,
            download_dir,
        })
    }

    /// Returns the current application version.
    pub fn current_version(&self) -> &Version {
        &self.current_version
    }

    /// Returns the current update state.
    pub fn state(&self) -> UpdateState {
        self.state.read().unwrap().clone()
    }

    /// Returns the current settings.
    pub fn settings(&self) -> UpdateSettings {
        self.settings.read().unwrap().clone()
    }

    /// Updates the settings.
    pub fn set_settings(&self, settings: UpdateSettings) {
        *self.settings.write().unwrap() = settings;
    }

    /// Checks if an update check is due based on the interval.
    pub fn is_check_due(&self) -> bool {
        let settings = self.settings.read().unwrap();

        if !settings.enabled {
            return false;
        }

        let Some(last_check) = &settings.last_check else {
            return true;
        };

        let Ok(last) = DateTime::parse_from_rfc3339(last_check) else {
            return true;
        };

        let elapsed = Utc::now().signed_duration_since(last.with_timezone(&Utc));
        elapsed.num_seconds() as u64 >= settings.check_interval_secs
    }

    /// Checks for updates.
    pub async fn check_for_updates(&self) -> Result<Option<UpdateInfo>> {
        let settings = self.settings.read().unwrap().clone();

        if !settings.enabled {
            return Err(UpdateError::Disabled);
        }

        // Update state
        *self.state.write().unwrap() = UpdateState::Checking;

        // Fetch latest release
        let release = match self
            .fetch_latest_release(&settings.repo_owner, &settings.repo_name)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                *self.state.write().unwrap() = UpdateState::Error(e.to_string());
                return Err(e);
            }
        };

        // Skip drafts
        if release.draft {
            *self.state.write().unwrap() = UpdateState::Idle;
            self.update_last_check();
            return Ok(None);
        }

        // Skip pre-releases if not enabled
        if release.prerelease && !settings.include_prereleases {
            *self.state.write().unwrap() = UpdateState::Idle;
            self.update_last_check();
            return Ok(None);
        }

        // Parse version (strip leading 'v' if present)
        let version_str = release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name);
        let release_version = match Version::parse(version_str) {
            Ok(v) => v,
            Err(e) => {
                *self.state.write().unwrap() = UpdateState::Error(e.to_string());
                return Err(UpdateError::VersionParse(e));
            }
        };

        // Check if newer
        if release_version <= self.current_version {
            *self.state.write().unwrap() = UpdateState::Idle;
            self.update_last_check();
            return Ok(None);
        }

        // Check if dismissed
        if let Some(dismissed) = &settings.dismissed_version {
            if dismissed == version_str {
                *self.state.write().unwrap() = UpdateState::Idle;
                self.update_last_check();
                return Ok(None);
            }
        }

        // Find compatible asset
        let (download_url, download_size, asset_name) = self
            .find_compatible_asset(&release.assets)
            .map(|a| {
                (
                    Some(a.browser_download_url.clone()),
                    Some(a.size),
                    Some(a.name.clone()),
                )
            })
            .unwrap_or((None, None, None));

        let update_info = UpdateInfo {
            version: version_str.to_string(),
            name: release.name,
            changelog: release.body,
            release_url: release.html_url,
            download_url,
            download_size,
            asset_name,
            published_at: release.published_at,
            prerelease: release.prerelease,
        };

        *self.state.write().unwrap() = UpdateState::Available(update_info.clone());
        self.update_last_check();

        Ok(Some(update_info))
    }

    /// Fetches the latest release from GitHub.
    async fn fetch_latest_release(&self, owner: &str, repo: &str) -> Result<GitHubRelease> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            owner, repo
        );

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?
            .error_for_status()?;

        let release: GitHubRelease = response.json().await?;
        Ok(release)
    }

    /// Finds a compatible asset for the current platform.
    fn find_compatible_asset<'a>(&self, assets: &'a [GitHubAsset]) -> Option<&'a GitHubAsset> {
        let platform_patterns = Self::platform_asset_patterns();

        for pattern in platform_patterns {
            for asset in assets {
                let name_lower = asset.name.to_lowercase();
                if name_lower.contains(pattern) {
                    return Some(asset);
                }
            }
        }

        None
    }

    /// Returns asset name patterns for the current platform.
    fn platform_asset_patterns() -> Vec<&'static str> {
        #[cfg(target_os = "macos")]
        {
            #[cfg(target_arch = "aarch64")]
            {
                vec!["macos-arm64.dmg", "darwin-arm64", "aarch64-apple-darwin"]
            }
            #[cfg(target_arch = "x86_64")]
            {
                vec!["macos-x64.dmg", "darwin-x64", "x86_64-apple-darwin"]
            }
            #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
            {
                vec!["macos", "darwin"]
            }
        }

        #[cfg(target_os = "windows")]
        {
            vec!["windows-x64.msi", "windows-x64.exe", "windows", "win64"]
        }

        #[cfg(target_os = "linux")]
        {
            vec![
                "linux-x64.appimage",
                "linux-x64.deb",
                "x86_64.appimage",
                "amd64.deb",
                "linux",
            ]
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            vec![]
        }
    }

    /// Downloads an update to the download directory.
    pub async fn download_update(&self, update_info: &UpdateInfo) -> Result<PathBuf> {
        let download_url = update_info
            .download_url
            .as_ref()
            .ok_or(UpdateError::NoCompatibleAsset)?;

        let asset_name = update_info
            .asset_name
            .as_ref()
            .ok_or(UpdateError::NoCompatibleAsset)?;

        // Create download directory if needed
        std::fs::create_dir_all(&self.download_dir)?;

        let dest_path = self.download_dir.join(asset_name);

        // Start download
        let response = self
            .client
            .get(download_url)
            .send()
            .await?
            .error_for_status()?;

        let total_size = response.content_length().unwrap_or(0);

        // Update state
        *self.state.write().unwrap() = UpdateState::Downloading(DownloadProgress {
            downloaded: 0,
            total: total_size,
            speed: 0,
        });

        // Download to file
        let bytes = response.bytes().await?;
        std::fs::write(&dest_path, &bytes)?;

        // Update state
        *self.state.write().unwrap() = UpdateState::Ready(dest_path.clone());

        Ok(dest_path)
    }

    /// Dismisses the current update (won't prompt again for this version).
    pub fn dismiss_update(&self, version: &str) {
        let mut settings = self.settings.write().unwrap();
        settings.dismissed_version = Some(version.to_string());
        *self.state.write().unwrap() = UpdateState::Idle;
    }

    /// Clears the dismissed version.
    pub fn clear_dismissed(&self) {
        let mut settings = self.settings.write().unwrap();
        settings.dismissed_version = None;
    }

    /// Updates the last check timestamp.
    fn update_last_check(&self) {
        let mut settings = self.settings.write().unwrap();
        settings.last_check = Some(Utc::now().to_rfc3339());
    }

    /// Opens the release page in the default browser.
    pub fn open_release_page(&self, update_info: &UpdateInfo) -> std::io::Result<()> {
        open::that(&update_info.release_url)
    }

    /// Gets instructions for installing the update based on the platform.
    pub fn get_install_instructions(&self, path: &std::path::Path) -> String {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("update");

        #[cfg(target_os = "macos")]
        {
            format!(
                "To install the update:\n\
                 1. Open {}\n\
                 2. Drag Aegis to your Applications folder\n\
                 3. Replace the existing version when prompted\n\
                 4. Restart Aegis",
                filename
            )
        }

        #[cfg(target_os = "windows")]
        {
            if filename.ends_with(".msi") {
                format!(
                    "To install the update:\n\
                     1. Close Aegis\n\
                     2. Run {}\n\
                     3. Follow the installer prompts\n\
                     4. Aegis will restart automatically",
                    filename
                )
            } else {
                format!(
                    "To install the update:\n\
                     1. Close Aegis\n\
                     2. Run {}\n\
                     3. Follow the installer prompts",
                    filename
                )
            }
        }

        #[cfg(target_os = "linux")]
        {
            if filename.ends_with(".deb") {
                format!(
                    "To install the update:\n\
                     1. Close Aegis\n\
                     2. Run: sudo dpkg -i {}\n\
                     3. Or double-click the file to open with your package manager\n\
                     4. Restart Aegis",
                    filename
                )
            } else if filename.ends_with(".rpm") {
                format!(
                    "To install the update:\n\
                     1. Close Aegis\n\
                     2. Run: sudo rpm -U {}\n\
                     3. Or double-click the file to open with your package manager\n\
                     4. Restart Aegis",
                    filename
                )
            } else if filename.ends_with(".AppImage") {
                format!(
                    "To install the update:\n\
                     1. Close Aegis\n\
                     2. Make executable: chmod +x {}\n\
                     3. Replace your existing AppImage\n\
                     4. Run the new version",
                    filename
                )
            } else {
                format!(
                    "To install the update:\n\
                     1. Close Aegis\n\
                     2. Extract and run {}\n\
                     3. Restart Aegis",
                    filename
                )
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            format!(
                "To install the update:\n\
                 1. Close Aegis\n\
                 2. Install {}\n\
                 3. Restart Aegis",
                filename
            )
        }
    }
}

/// Creates an UpdateManager wrapped in an Arc for sharing.
pub fn create_update_manager(download_dir: PathBuf) -> Result<Arc<UpdateManager>> {
    Ok(Arc::new(UpdateManager::new(download_dir)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_update_settings_default() {
        let settings = UpdateSettings::default();
        assert!(settings.enabled);
        assert!(!settings.include_prereleases);
        assert!(!settings.auto_download);
        assert_eq!(settings.check_interval_secs, DEFAULT_CHECK_INTERVAL_SECS);
    }

    #[test]
    fn test_download_progress_percent() {
        let progress = DownloadProgress {
            downloaded: 50,
            total: 100,
            speed: 1000,
        };
        assert!((progress.percent() - 50.0).abs() < f64::EPSILON);

        let zero_progress = DownloadProgress {
            downloaded: 0,
            total: 0,
            speed: 0,
        };
        assert!((zero_progress.percent() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_update_manager_new() {
        let temp_dir = env::temp_dir().join("aegis_update_test");
        let manager = UpdateManager::new(temp_dir).unwrap();

        assert!(matches!(manager.state(), UpdateState::Idle));
        assert!(manager.settings().enabled);
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::parse("0.1.0").unwrap();
        let v2 = Version::parse("0.2.0").unwrap();
        let v3 = Version::parse("1.0.0").unwrap();

        assert!(v2 > v1);
        assert!(v3 > v2);
        assert!(v3 > v1);
    }

    #[test]
    fn test_is_check_due_no_last_check() {
        let temp_dir = env::temp_dir().join("aegis_update_test2");
        let manager = UpdateManager::new(temp_dir).unwrap();

        // No last check means check is due
        assert!(manager.is_check_due());
    }

    #[test]
    fn test_is_check_due_disabled() {
        let temp_dir = env::temp_dir().join("aegis_update_test3");
        let mut settings = UpdateSettings::default();
        settings.enabled = false;
        let manager = UpdateManager::with_settings(temp_dir, settings).unwrap();

        // Disabled means check is not due
        assert!(!manager.is_check_due());
    }

    #[test]
    fn test_dismiss_update() {
        let temp_dir = env::temp_dir().join("aegis_update_test4");
        let manager = UpdateManager::new(temp_dir).unwrap();

        manager.dismiss_update("1.0.0");

        let settings = manager.settings();
        assert_eq!(settings.dismissed_version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_clear_dismissed() {
        let temp_dir = env::temp_dir().join("aegis_update_test5");
        let manager = UpdateManager::new(temp_dir).unwrap();

        manager.dismiss_update("1.0.0");
        manager.clear_dismissed();

        let settings = manager.settings();
        assert!(settings.dismissed_version.is_none());
    }

    #[test]
    fn test_platform_patterns() {
        let patterns = UpdateManager::platform_asset_patterns();
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_get_install_instructions() {
        let temp_dir = env::temp_dir().join("aegis_update_test6");
        let manager = UpdateManager::new(temp_dir).unwrap();

        #[cfg(target_os = "windows")]
        {
            let instructions = manager.get_install_instructions(std::path::Path::new("Aegis.msi"));
            assert!(instructions.contains("Run Aegis.msi"));
        }

        #[cfg(target_os = "macos")]
        {
            let instructions = manager.get_install_instructions(std::path::Path::new("Aegis.dmg"));
            assert!(instructions.contains("Applications"));
        }

        #[cfg(target_os = "linux")]
        {
            let instructions = manager.get_install_instructions(std::path::Path::new("aegis.deb"));
            assert!(instructions.contains("dpkg"));
        }
    }
}

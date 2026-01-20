//! ML Model and Runtime Downloader.
//!
//! Downloads ONNX Runtime and ML models on first use or via settings.
//! Supports progress tracking for UI integration.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use directories::ProjectDirs;

/// Download progress callback type (uses Arc for Clone support).
pub type ProgressCallback = Arc<dyn Fn(DownloadProgress) + Send + Sync>;

/// Download progress information.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Current step description.
    pub step: String,
    /// Bytes downloaded so far.
    pub downloaded: u64,
    /// Total bytes to download (if known).
    pub total: Option<u64>,
    /// Whether the download is complete.
    pub complete: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

impl DownloadProgress {
    /// Creates a new progress update.
    pub fn new(step: &str, downloaded: u64, total: Option<u64>) -> Self {
        Self {
            step: step.to_string(),
            downloaded,
            total,
            complete: false,
            error: None,
        }
    }

    /// Creates a completion progress.
    pub fn complete(step: &str) -> Self {
        Self {
            step: step.to_string(),
            downloaded: 0,
            total: None,
            complete: true,
            error: None,
        }
    }

    /// Creates an error progress.
    pub fn error(step: &str, error: &str) -> Self {
        Self {
            step: step.to_string(),
            downloaded: 0,
            total: None,
            complete: false,
            error: Some(error.to_string()),
        }
    }

    /// Returns progress as a percentage (0-100).
    pub fn percentage(&self) -> Option<u8> {
        self.total.map(|t| {
            if t == 0 {
                100
            } else {
                ((self.downloaded as f64 / t as f64) * 100.0).min(100.0) as u8
            }
        })
    }
}

/// Error types for model downloading.
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("ZIP extraction error: {0}")]
    Zip(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Model not found: {0}")]
    NotFound(String),
}

/// ONNX Runtime version to download.
const ONNX_RUNTIME_VERSION: &str = "1.23.2";

/// ONNX Runtime download URL for Windows x64.
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const ONNX_RUNTIME_URL: &str = "https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-win-x64-1.23.2.zip";

/// ONNX Runtime download URL for Linux x64.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const ONNX_RUNTIME_URL: &str = "https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-linux-x64-1.23.2.tgz";

/// ONNX Runtime download URL for macOS x64.
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const ONNX_RUNTIME_URL: &str = "https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-osx-x86_64-1.23.2.tgz";

/// ONNX Runtime download URL for macOS ARM64.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const ONNX_RUNTIME_URL: &str = "https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-osx-arm64-1.23.2.tgz";

/// Fallback for unsupported platforms.
#[cfg(not(any(
    all(target_os = "windows", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "macos", target_arch = "x86_64"),
    all(target_os = "macos", target_arch = "aarch64"),
)))]
const ONNX_RUNTIME_URL: &str = "";

/// NSFW model from Hugging Face (onnx-community/nsfw-image-detector-ONNX - 5-class model).
/// Classes: drawings, hentai, neutral, porn, sexy
const NSFW_MODEL_URL: &str = "https://huggingface.co/onnx-community/nsfw-image-detector-ONNX/resolve/main/onnx/model.onnx";

/// Model downloader for ONNX Runtime and ML models.
pub struct ModelDownloader {
    /// Directory to store downloaded files.
    data_dir: PathBuf,
    /// Directory for models.
    models_dir: PathBuf,
    /// Directory for runtime libraries.
    lib_dir: PathBuf,
}

impl ModelDownloader {
    /// Creates a new model downloader.
    pub fn new() -> Option<Self> {
        let project_dirs = ProjectDirs::from("", "aegis", "Aegis")?;
        let data_dir = project_dirs.data_dir().to_path_buf();
        let models_dir = data_dir.join("models");
        let lib_dir = data_dir.join("lib");

        Some(Self {
            data_dir,
            models_dir,
            lib_dir,
        })
    }

    /// Returns the data directory path.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Returns the models directory path.
    pub fn models_dir(&self) -> &Path {
        &self.models_dir
    }

    /// Returns the lib directory path.
    pub fn lib_dir(&self) -> &Path {
        &self.lib_dir
    }

    /// Returns the path to the ONNX Runtime library.
    #[cfg(target_os = "windows")]
    pub fn onnx_runtime_path(&self) -> PathBuf {
        self.lib_dir.join("onnxruntime.dll")
    }

    #[cfg(target_os = "linux")]
    pub fn onnx_runtime_path(&self) -> PathBuf {
        self.lib_dir.join("libonnxruntime.so")
    }

    #[cfg(target_os = "macos")]
    pub fn onnx_runtime_path(&self) -> PathBuf {
        self.lib_dir.join("libonnxruntime.dylib")
    }

    /// Returns the path to the NSFW model.
    pub fn nsfw_model_path(&self) -> PathBuf {
        self.models_dir.join("nsfw_image_classifier.onnx")
    }

    /// Checks if ONNX Runtime is installed.
    pub fn is_onnx_runtime_installed(&self) -> bool {
        self.onnx_runtime_path().exists()
    }

    /// Checks if the NSFW model is installed.
    pub fn is_nsfw_model_installed(&self) -> bool {
        self.nsfw_model_path().exists()
    }

    /// Checks if all ML dependencies are installed.
    pub fn is_ml_ready(&self) -> bool {
        self.is_onnx_runtime_installed() && self.is_nsfw_model_installed()
    }

    /// Downloads ONNX Runtime if not already installed.
    pub async fn ensure_onnx_runtime(
        &self,
        progress: Option<ProgressCallback>,
    ) -> Result<PathBuf, DownloadError> {
        if self.is_onnx_runtime_installed() {
            if let Some(ref cb) = progress {
                cb(DownloadProgress::complete("ONNX Runtime already installed"));
            }
            return Ok(self.onnx_runtime_path());
        }

        self.download_onnx_runtime(progress).await
    }

    /// Downloads the NSFW model if not already installed.
    pub async fn ensure_nsfw_model(
        &self,
        progress: Option<ProgressCallback>,
    ) -> Result<PathBuf, DownloadError> {
        if self.is_nsfw_model_installed() {
            if let Some(ref cb) = progress {
                cb(DownloadProgress::complete("NSFW model already installed"));
            }
            return Ok(self.nsfw_model_path());
        }

        self.download_nsfw_model(progress).await
    }

    /// Ensures all ML dependencies are installed.
    pub async fn ensure_all(
        &self,
        progress: Option<ProgressCallback>,
    ) -> Result<(), DownloadError> {
        self.ensure_onnx_runtime(progress.clone()).await?;
        self.ensure_nsfw_model(progress).await?;
        Ok(())
    }

    /// Downloads ONNX Runtime.
    async fn download_onnx_runtime(
        &self,
        progress: Option<ProgressCallback>,
    ) -> Result<PathBuf, DownloadError> {
        if ONNX_RUNTIME_URL.is_empty() {
            return Err(DownloadError::NotFound(
                "ONNX Runtime not available for this platform".to_string(),
            ));
        }

        // Create lib directory
        fs::create_dir_all(&self.lib_dir)?;

        if let Some(ref cb) = progress {
            cb(DownloadProgress::new(
                &format!("Downloading ONNX Runtime v{}...", ONNX_RUNTIME_VERSION),
                0,
                None,
            ));
        }

        // Download the archive
        let response = reqwest::get(ONNX_RUNTIME_URL)
            .await
            .map_err(|e| DownloadError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DownloadError::Network(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let total_size = response.content_length();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| DownloadError::Network(e.to_string()))?;

        if let Some(ref cb) = progress {
            cb(DownloadProgress::new(
                "Extracting ONNX Runtime...",
                bytes.len() as u64,
                total_size,
            ));
        }

        // Extract the library
        #[cfg(target_os = "windows")]
        {
            self.extract_zip(&bytes, "onnxruntime.dll")?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            self.extract_tgz(&bytes)?;
        }

        if let Some(ref cb) = progress {
            cb(DownloadProgress::complete("ONNX Runtime installed"));
        }

        Ok(self.onnx_runtime_path())
    }

    /// Downloads the NSFW model.
    async fn download_nsfw_model(
        &self,
        progress: Option<ProgressCallback>,
    ) -> Result<PathBuf, DownloadError> {
        // Create models directory
        fs::create_dir_all(&self.models_dir)?;

        if let Some(ref cb) = progress {
            cb(DownloadProgress::new("Downloading NSFW model...", 0, None));
        }

        // Download the model
        let response = reqwest::get(NSFW_MODEL_URL)
            .await
            .map_err(|e| DownloadError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DownloadError::Network(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let total_size = response.content_length();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| DownloadError::Network(e.to_string()))?;

        if let Some(ref cb) = progress {
            cb(DownloadProgress::new(
                "Saving NSFW model...",
                bytes.len() as u64,
                total_size,
            ));
        }

        // Save the model
        let model_path = self.nsfw_model_path();
        let mut file = File::create(&model_path)?;
        file.write_all(&bytes)?;

        if let Some(ref cb) = progress {
            cb(DownloadProgress::complete("NSFW model installed"));
        }

        Ok(model_path)
    }

    /// Extracts a DLL from a ZIP archive (Windows).
    #[cfg(target_os = "windows")]
    fn extract_zip(&self, data: &[u8], dll_name: &str) -> Result<(), DownloadError> {
        use std::io::{Cursor, Read};
        use zip::ZipArchive;

        let cursor = Cursor::new(data);
        let mut archive =
            ZipArchive::new(cursor).map_err(|e| DownloadError::Zip(e.to_string()))?;

        // Find and extract the DLL
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| DownloadError::Zip(e.to_string()))?;
            let name = file.name().to_string();

            if name.ends_with(dll_name) {
                let dest_path = self.lib_dir.join(dll_name);
                let mut dest_file = File::create(&dest_path)?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                dest_file.write_all(&buffer)?;
                return Ok(());
            }
        }

        Err(DownloadError::Zip(format!(
            "{} not found in archive",
            dll_name
        )))
    }

    /// Extracts libraries from a tar.gz archive (Linux/macOS).
    #[cfg(not(target_os = "windows"))]
    fn extract_tgz(&self, data: &[u8]) -> Result<(), DownloadError> {
        use flate2::read::GzDecoder;
        use std::io::Cursor;
        use tar::Archive;

        let cursor = Cursor::new(data);
        let decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);

        #[cfg(target_os = "linux")]
        let lib_name = "libonnxruntime.so";
        #[cfg(target_os = "macos")]
        let lib_name = "libonnxruntime.dylib";

        for entry in archive
            .entries()
            .map_err(|e| DownloadError::Zip(e.to_string()))?
        {
            let mut entry = entry.map_err(|e| DownloadError::Zip(e.to_string()))?;
            let path = entry
                .path()
                .map_err(|e| DownloadError::Zip(e.to_string()))?;

            if path
                .file_name()
                .map(|n| n.to_string_lossy().starts_with(lib_name))
                .unwrap_or(false)
            {
                let dest_path = self.lib_dir.join(lib_name);
                let mut dest_file = File::create(&dest_path)?;
                std::io::copy(&mut entry, &mut dest_file)?;
                return Ok(());
            }
        }

        Err(DownloadError::Zip(format!(
            "{} not found in archive",
            lib_name
        )))
    }

    /// Gets the environment variable name for ONNX Runtime library path.
    pub fn onnx_lib_env_var() -> &'static str {
        "ORT_DYLIB_PATH"
    }

    /// Sets up the environment for ONNX Runtime.
    pub fn setup_environment(&self) -> bool {
        if self.is_onnx_runtime_installed() {
            let lib_path = self.onnx_runtime_path();
            std::env::set_var(Self::onnx_lib_env_var(), &lib_path);
            tracing::info!("Set {} to {:?}", Self::onnx_lib_env_var(), lib_path);
            true
        } else {
            false
        }
    }
}

impl Default for ModelDownloader {
    fn default() -> Self {
        Self::new().expect("Failed to create ModelDownloader")
    }
}

/// Status of ML dependencies.
#[derive(Debug, Clone, PartialEq)]
pub enum MlStatus {
    /// All dependencies are installed and ready.
    Ready,
    /// ONNX Runtime is missing.
    MissingRuntime,
    /// NSFW model is missing.
    MissingModel,
    /// Both are missing.
    MissingAll,
    /// Currently downloading.
    Downloading { step: String, progress: Option<u8> },
    /// Download failed.
    Failed { error: String },
}

impl MlStatus {
    /// Returns true if ML is ready to use.
    pub fn is_ready(&self) -> bool {
        matches!(self, MlStatus::Ready)
    }

    /// Returns a human-readable description.
    pub fn description(&self) -> String {
        match self {
            MlStatus::Ready => "Image filtering ready".to_string(),
            MlStatus::MissingRuntime => "ONNX Runtime not installed".to_string(),
            MlStatus::MissingModel => "NSFW model not installed".to_string(),
            MlStatus::MissingAll => "ML dependencies not installed".to_string(),
            MlStatus::Downloading { step, progress } => {
                if let Some(p) = progress {
                    format!("{} ({}%)", step, p)
                } else {
                    step.clone()
                }
            }
            MlStatus::Failed { error } => format!("Download failed: {}", error),
        }
    }
}

/// Gets the current ML status.
pub fn get_ml_status() -> MlStatus {
    let Some(downloader) = ModelDownloader::new() else {
        return MlStatus::Failed {
            error: "Failed to initialize downloader".to_string(),
        };
    };

    let has_runtime = downloader.is_onnx_runtime_installed();
    let has_model = downloader.is_nsfw_model_installed();

    match (has_runtime, has_model) {
        (true, true) => MlStatus::Ready,
        (false, true) => MlStatus::MissingRuntime,
        (true, false) => MlStatus::MissingModel,
        (false, false) => MlStatus::MissingAll,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_progress_percentage() {
        let p = DownloadProgress::new("test", 50, Some(100));
        assert_eq!(p.percentage(), Some(50));

        let p = DownloadProgress::new("test", 0, Some(100));
        assert_eq!(p.percentage(), Some(0));

        let p = DownloadProgress::new("test", 100, Some(100));
        assert_eq!(p.percentage(), Some(100));

        let p = DownloadProgress::new("test", 50, None);
        assert_eq!(p.percentage(), None);
    }

    #[test]
    fn ml_status_descriptions() {
        assert!(MlStatus::Ready.is_ready());
        assert!(!MlStatus::MissingAll.is_ready());

        let downloading = MlStatus::Downloading {
            step: "Downloading...".to_string(),
            progress: Some(50),
        };
        assert_eq!(downloading.description(), "Downloading... (50%)");
    }

    #[test]
    fn model_downloader_paths() {
        if let Some(downloader) = ModelDownloader::new() {
            assert!(downloader.models_dir().ends_with("models"));
            assert!(downloader.lib_dir().ends_with("lib"));

            #[cfg(target_os = "windows")]
            assert!(downloader
                .onnx_runtime_path()
                .to_string_lossy()
                .ends_with("onnxruntime.dll"));
        }
    }
}

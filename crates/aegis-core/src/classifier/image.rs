//! NSFW Image Classifier (F033).
//!
//! Uses an ONNX Vision Transformer model to detect NSFW/explicit content
//! in images. Designed to run in <100ms on CPU.

#[cfg(feature = "ml")]
use std::path::Path;
#[cfg(feature = "ml")]
use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Result of NSFW image classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NsfwImageResult {
    /// Probability that the image is safe/SFW (0.0 to 1.0).
    pub sfw_probability: f32,
    /// Probability that the image is NSFW (0.0 to 1.0).
    pub nsfw_probability: f32,
    /// Classification duration in microseconds.
    pub duration_us: u64,
    /// Image dimensions (width, height) that were classified.
    pub image_dimensions: (u32, u32),
}

impl NsfwImageResult {
    /// Returns true if the image is classified as NSFW (probability > threshold).
    pub fn is_nsfw(&self, threshold: f32) -> bool {
        self.nsfw_probability > threshold
    }

    /// Returns true if the image is classified as safe.
    pub fn is_safe(&self, threshold: f32) -> bool {
        !self.is_nsfw(threshold)
    }
}

/// Error types for NSFW image classifier.
#[derive(Debug, thiserror::Error)]
pub enum NsfwImageError {
    /// Model file not found.
    #[error("Model file not found: {0}")]
    ModelNotFound(String),

    /// ONNX runtime error.
    #[cfg(feature = "ml")]
    #[error("ONNX runtime error: {0}")]
    OrtError(#[from] ort::Error),

    /// Image processing error.
    #[error("Image processing error: {0}")]
    ImageError(String),

    /// Inference error.
    #[error("Inference error: {0}")]
    InferenceError(String),

    /// Image too large.
    #[error("Image too large: {0} bytes (max: {1} bytes)")]
    ImageTooLarge(usize, usize),

    /// Invalid image format.
    #[error("Invalid image format: {0}")]
    InvalidFormat(String),

    /// ML feature not enabled.
    #[error("ML feature not enabled - rebuild with --features ml")]
    MlNotEnabled,
}

#[cfg(feature = "ml")]
impl From<image::ImageError> for NsfwImageError {
    fn from(e: image::ImageError) -> Self {
        NsfwImageError::ImageError(e.to_string())
    }
}

/// Configuration for the NSFW image classifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NsfwImageConfig {
    /// Path to the ONNX model file.
    pub model_path: String,
    /// Default threshold for NSFW classification (default: 0.5).
    pub default_threshold: f32,
    /// Maximum image size in bytes (default: 10MB).
    pub max_image_size: usize,
    /// Model input size (default: 224x224 for ViT models).
    pub input_size: u32,
}

impl Default for NsfwImageConfig {
    fn default() -> Self {
        // Try to get model path from downloader, fall back to local path
        let model_path = match crate::model_downloader::ModelDownloader::new() {
            Some(d) => {
                // Set up ONNX environment so the runtime DLL can be found
                d.setup_environment();
                let path = d.nsfw_model_path().to_string_lossy().to_string();
                tracing::info!("NSFW model path from downloader: {}", path);
                path
            }
            None => {
                tracing::warn!("ModelDownloader::new() returned None, using fallback path");
                "models/nsfw_image_classifier.onnx".to_string()
            }
        };

        Self {
            model_path,
            default_threshold: 0.7, // Higher threshold = fewer false positives
            max_image_size: 10 * 1024 * 1024, // 10 MB
            input_size: 224, // onnx-community/nsfw-image-detector-ONNX uses 224x224 input
        }
    }
}

impl NsfwImageConfig {
    /// Creates a config with the model path from the downloader.
    pub fn from_downloader() -> Option<Self> {
        let downloader = crate::model_downloader::ModelDownloader::new()?;

        // Set up ONNX environment to find the runtime
        downloader.setup_environment();

        Some(Self {
            model_path: downloader.nsfw_model_path().to_string_lossy().to_string(),
            ..Default::default()
        })
    }
}

/// Age-based NSFW threshold presets.
/// Note: Higher thresholds = fewer false positives (less sensitive).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum NsfwThresholdPreset {
    /// Child profile (< 13): Aggressive blocking (0.5).
    Child,
    /// Teen profile (13-17): Balanced blocking (0.7).
    #[default]
    Teen,
    /// Adult profile (18+): Permissive blocking (0.85).
    Adult,
    /// Custom threshold.
    Custom(f32),
}

impl NsfwThresholdPreset {
    /// Returns the threshold value for this preset.
    pub fn threshold(&self) -> f32 {
        match self {
            NsfwThresholdPreset::Child => 0.5,
            NsfwThresholdPreset::Teen => 0.7,
            NsfwThresholdPreset::Adult => 0.85,
            NsfwThresholdPreset::Custom(t) => t.clamp(0.0, 1.0),
        }
    }

    /// Creates a preset from an age.
    pub fn from_age(age: u8) -> Self {
        match age {
            0..=12 => NsfwThresholdPreset::Child,
            13..=17 => NsfwThresholdPreset::Teen,
            _ => NsfwThresholdPreset::Adult,
        }
    }
}

/// ML-based NSFW image classifier using Vision Transformer model.
///
/// This classifier runs ONNX inference to detect explicit/NSFW content
/// in images with high accuracy. Supports common image formats.
#[cfg(feature = "ml")]
pub struct NsfwImageClassifier {
    session: ort::session::Session,
    config: NsfwImageConfig,
}

#[cfg(feature = "ml")]
impl NsfwImageClassifier {
    /// Creates a new NSFW image classifier by loading the ONNX model.
    ///
    /// Returns an error if the model file is not found.
    pub fn new(config: NsfwImageConfig) -> Result<Self, NsfwImageError> {
        use ort::session::{builder::GraphOptimizationLevel, Session};

        // Check if model file exists
        if !Path::new(&config.model_path).exists() {
            return Err(NsfwImageError::ModelNotFound(config.model_path.clone()));
        }

        // Load ONNX model with optimizations
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(2)? // Use 2 threads for image inference
            .commit_from_file(&config.model_path)?;

        Ok(Self { session, config })
    }

    /// Loads the classifier from default paths.
    pub fn load_default() -> Result<Self, NsfwImageError> {
        Self::new(NsfwImageConfig::default())
    }

    /// Attempts to load the classifier, returning None if files don't exist.
    ///
    /// This is the preferred way to initialize when graceful fallback is desired.
    pub fn try_load(config: NsfwImageConfig) -> Option<Self> {
        Self::new(config).ok()
    }

    /// Classifies image bytes and returns SFW/NSFW probabilities.
    ///
    /// Supports JPEG, PNG, WebP, and GIF (first frame) formats.
    pub fn classify_bytes(&mut self, image_data: &[u8]) -> Result<NsfwImageResult, NsfwImageError> {
        // Check image size
        if image_data.len() > self.config.max_image_size {
            return Err(NsfwImageError::ImageTooLarge(
                image_data.len(),
                self.config.max_image_size,
            ));
        }

        let start = Instant::now();

        // Decode image
        let img = image::load_from_memory(image_data)?;
        let original_dims = (img.width(), img.height());

        // Preprocess: resize to model input size and convert to RGB
        let resized = img.resize_exact(
            self.config.input_size,
            self.config.input_size,
            image::imageops::FilterType::Triangle,
        );
        let rgb_img = resized.to_rgb8();

        // Convert to normalized float tensor [1, 3, H, W]
        // Normalization values for ImageNet-pretrained models
        let mean = [0.485, 0.456, 0.406];
        let std = [0.229, 0.224, 0.225];

        let input_size = self.config.input_size as usize;
        let mut tensor_data = vec![0.0f32; 3 * input_size * input_size];

        for (y, row) in rgb_img.rows().enumerate() {
            for (x, pixel) in row.enumerate() {
                let r = (pixel[0] as f32 / 255.0 - mean[0]) / std[0];
                let g = (pixel[1] as f32 / 255.0 - mean[1]) / std[1];
                let b = (pixel[2] as f32 / 255.0 - mean[2]) / std[2];

                // CHW format: [channel][height][width]
                tensor_data[0 * input_size * input_size + y * input_size + x] = r;
                tensor_data[1 * input_size * input_size + y * input_size + x] = g;
                tensor_data[2 * input_size * input_size + y * input_size + x] = b;
            }
        }

        // Create ONNX tensor with shape [1, 3, H, W]
        let input_tensor = ort::value::Tensor::from_array((
            [1usize, 3, input_size, input_size],
            tensor_data.into_boxed_slice(),
        ))?;

        // Run inference
        let outputs = self.session.run(ort::inputs![
            "pixel_values" => input_tensor
        ])?;

        // Extract logits from output
        let logits_tensor = outputs["logits"].try_extract_tensor::<f32>().map_err(|e| {
            NsfwImageError::InferenceError(format!("Failed to extract logits: {}", e))
        })?;

        let logits_data = logits_tensor.1;

        // Handle both 2-class and 5-class models
        let (sfw_prob, nsfw_prob) = if logits_data.len() >= 5 {
            // 5-class model: drawings(0), hentai(1), neutral(2), porn(3), sexy(4)
            let probs = softmax_multi(&[
                logits_data[0], // drawings
                logits_data[1], // hentai
                logits_data[2], // neutral
                logits_data[3], // porn
                logits_data[4], // sexy
            ]);
            // SFW = drawings + neutral, NSFW = hentai + porn + sexy
            let sfw = probs[0] + probs[2];
            let nsfw = probs[1] + probs[3] + probs[4];
            (sfw, nsfw)
        } else if logits_data.len() >= 2 {
            // 2-class model: SFW(0), NSFW(1)
            softmax(logits_data[0], logits_data[1])
        } else {
            return Err(NsfwImageError::InferenceError(format!(
                "Expected at least 2 output classes, got {}",
                logits_data.len()
            )));
        };

        let duration_us = start.elapsed().as_micros() as u64;

        Ok(NsfwImageResult {
            sfw_probability: sfw_prob,
            nsfw_probability: nsfw_prob,
            duration_us,
            image_dimensions: original_dims,
        })
    }

    /// Classifies an image from a file path.
    pub fn classify_file(&mut self, path: &Path) -> Result<NsfwImageResult, NsfwImageError> {
        let image_data = std::fs::read(path)
            .map_err(|e| NsfwImageError::ImageError(format!("Failed to read file: {}", e)))?;
        self.classify_bytes(&image_data)
    }

    /// Classifies a base64-encoded image.
    pub fn classify_base64(
        &mut self,
        base64_data: &str,
    ) -> Result<NsfwImageResult, NsfwImageError> {
        use base64::{engine::general_purpose::STANDARD, Engine};

        // Handle data URI prefix
        let base64_str = if let Some(pos) = base64_data.find(",") {
            &base64_data[pos + 1..]
        } else {
            base64_data
        };

        let image_data = STANDARD
            .decode(base64_str)
            .map_err(|e| NsfwImageError::InvalidFormat(format!("Invalid base64: {}", e)))?;

        self.classify_bytes(&image_data)
    }

    /// Returns the configured default threshold.
    pub fn default_threshold(&self) -> f32 {
        self.config.default_threshold
    }

    /// Returns the maximum allowed image size.
    pub fn max_image_size(&self) -> usize {
        self.config.max_image_size
    }

    /// Returns the model input size.
    pub fn input_size(&self) -> u32 {
        self.config.input_size
    }
}

/// Computes softmax for two values.
#[cfg(feature = "ml")]
fn softmax(a: f32, b: f32) -> (f32, f32) {
    let max = a.max(b);
    let exp_a = (a - max).exp();
    let exp_b = (b - max).exp();
    let sum = exp_a + exp_b;
    (exp_a / sum, exp_b / sum)
}

/// Computes softmax for multiple values (5-class model support).
#[cfg(feature = "ml")]
fn softmax_multi(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp_vals: Vec<f32> = logits.iter().map(|x| (x - max).exp()).collect();
    let sum: f32 = exp_vals.iter().sum();
    exp_vals.iter().map(|x| x / sum).collect()
}

/// Stub classifier when ML feature is not enabled.
///
/// Always returns an error indicating ML is not available.
#[cfg(not(feature = "ml"))]
pub struct NsfwImageClassifier {
    _config: NsfwImageConfig,
}

#[cfg(not(feature = "ml"))]
impl NsfwImageClassifier {
    /// Creates a stub classifier (ML feature not enabled).
    pub fn new(_config: NsfwImageConfig) -> Result<Self, NsfwImageError> {
        Err(NsfwImageError::MlNotEnabled)
    }

    /// Loads the classifier from default paths (ML feature not enabled).
    pub fn load_default() -> Result<Self, NsfwImageError> {
        Err(NsfwImageError::MlNotEnabled)
    }

    /// Attempts to load the classifier (always returns None when ML is disabled).
    pub fn try_load(_config: NsfwImageConfig) -> Option<Self> {
        None
    }

    /// Classifies image bytes (ML feature not enabled).
    pub fn classify_bytes(
        &mut self,
        _image_data: &[u8],
    ) -> Result<NsfwImageResult, NsfwImageError> {
        Err(NsfwImageError::MlNotEnabled)
    }

    /// Classifies an image from a file path (ML feature not enabled).
    pub fn classify_file(
        &mut self,
        _path: &std::path::Path,
    ) -> Result<NsfwImageResult, NsfwImageError> {
        Err(NsfwImageError::MlNotEnabled)
    }

    /// Classifies a base64-encoded image (ML feature not enabled).
    pub fn classify_base64(
        &mut self,
        _base64_data: &str,
    ) -> Result<NsfwImageResult, NsfwImageError> {
        Err(NsfwImageError::MlNotEnabled)
    }

    /// Returns the configured default threshold.
    pub fn default_threshold(&self) -> f32 {
        0.5
    }

    /// Returns the maximum allowed image size.
    pub fn max_image_size(&self) -> usize {
        10 * 1024 * 1024
    }

    /// Returns the model input size.
    pub fn input_size(&self) -> u32 {
        224
    }
}

/// Lazy-loaded NSFW image classifier singleton.
///
/// Use this for efficient resource management - the model is only loaded
/// when the first image needs to be classified.
pub struct LazyNsfwClassifier {
    classifier: Option<NsfwImageClassifier>,
    config: NsfwImageConfig,
    load_attempted: bool,
}

impl LazyNsfwClassifier {
    /// Creates a new lazy classifier with the given config.
    pub fn new(config: NsfwImageConfig) -> Self {
        Self {
            classifier: None,
            config,
            load_attempted: false,
        }
    }

    /// Creates a lazy classifier with default config.
    pub fn with_defaults() -> Self {
        Self::new(NsfwImageConfig::default())
    }

    /// Returns true if the classifier is loaded and ready.
    pub fn is_loaded(&self) -> bool {
        self.classifier.is_some()
    }

    /// Returns true if loading was attempted (whether successful or not).
    pub fn load_attempted(&self) -> bool {
        self.load_attempted
    }

    /// Attempts to load the classifier if not already loaded.
    ///
    /// Returns true if classifier is available (was already loaded or successfully loaded now).
    /// Will retry loading if the model file now exists (e.g., after download completes).
    pub fn ensure_loaded(&mut self) -> bool {
        if self.classifier.is_some() {
            return true;
        }

        // If we previously tried and failed, check if model file now exists before retrying
        if self.load_attempted {
            let model_path = std::path::Path::new(&self.config.model_path);
            if !model_path.exists() {
                // Model still not available, don't spam logs
                return false;
            }
            // Model file now exists! Reset flag to allow retry
            tracing::info!(
                "NSFW model file now exists at {}, retrying load",
                self.config.model_path
            );
            self.load_attempted = false;
        }

        self.load_attempted = true;
        self.classifier = NsfwImageClassifier::try_load(self.config.clone());

        if self.classifier.is_none() {
            tracing::warn!(
                "NSFW image classifier not available - model file not found at {}",
                self.config.model_path
            );
        } else {
            tracing::info!("NSFW image classifier loaded successfully");
        }

        self.classifier.is_some()
    }

    /// Classifies image bytes if the classifier is available.
    ///
    /// Returns None if the classifier could not be loaded.
    pub fn classify_bytes(
        &mut self,
        image_data: &[u8],
    ) -> Option<Result<NsfwImageResult, NsfwImageError>> {
        if !self.ensure_loaded() {
            return None;
        }

        Some(self.classifier.as_mut().unwrap().classify_bytes(image_data))
    }

    /// Classifies a base64-encoded image if the classifier is available.
    ///
    /// Returns None if the classifier could not be loaded.
    pub fn classify_base64(
        &mut self,
        base64_data: &str,
    ) -> Option<Result<NsfwImageResult, NsfwImageError>> {
        if !self.ensure_loaded() {
            return None;
        }

        Some(
            self.classifier
                .as_mut()
                .unwrap()
                .classify_base64(base64_data),
        )
    }

    /// Returns the config.
    pub fn config(&self) -> &NsfwImageConfig {
        &self.config
    }
}

impl Default for LazyNsfwClassifier {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nsfw_result_is_nsfw() {
        let result = NsfwImageResult {
            sfw_probability: 0.2,
            nsfw_probability: 0.8,
            duration_us: 100,
            image_dimensions: (100, 100),
        };
        assert!(result.is_nsfw(0.5));
        assert!(result.is_nsfw(0.7));
        assert!(!result.is_nsfw(0.9));
    }

    #[test]
    fn nsfw_result_is_safe() {
        let result = NsfwImageResult {
            sfw_probability: 0.9,
            nsfw_probability: 0.1,
            duration_us: 100,
            image_dimensions: (100, 100),
        };
        assert!(result.is_safe(0.5));
        assert!(result.is_safe(0.2));
        assert!(!result.is_safe(0.05));
    }

    #[test]
    fn threshold_presets() {
        assert_eq!(NsfwThresholdPreset::Child.threshold(), 0.5);
        assert_eq!(NsfwThresholdPreset::Teen.threshold(), 0.7);
        assert_eq!(NsfwThresholdPreset::Adult.threshold(), 0.85);
        assert_eq!(NsfwThresholdPreset::Custom(0.6).threshold(), 0.6);
        assert_eq!(NsfwThresholdPreset::Custom(1.5).threshold(), 1.0); // Clamped
        assert_eq!(NsfwThresholdPreset::Custom(-0.5).threshold(), 0.0); // Clamped
    }

    #[test]
    fn threshold_from_age() {
        assert_eq!(NsfwThresholdPreset::from_age(5), NsfwThresholdPreset::Child);
        assert_eq!(
            NsfwThresholdPreset::from_age(12),
            NsfwThresholdPreset::Child
        );
        assert_eq!(NsfwThresholdPreset::from_age(13), NsfwThresholdPreset::Teen);
        assert_eq!(NsfwThresholdPreset::from_age(17), NsfwThresholdPreset::Teen);
        assert_eq!(
            NsfwThresholdPreset::from_age(18),
            NsfwThresholdPreset::Adult
        );
        assert_eq!(
            NsfwThresholdPreset::from_age(30),
            NsfwThresholdPreset::Adult
        );
    }

    #[test]
    fn config_default_values() {
        let config = NsfwImageConfig::default();
        assert_eq!(config.default_threshold, 0.7);
        assert_eq!(config.max_image_size, 10 * 1024 * 1024);
        assert_eq!(config.input_size, 224);
    }

    #[test]
    fn try_load_returns_none_when_model_missing() {
        let config = NsfwImageConfig {
            model_path: "nonexistent/model.onnx".to_string(),
            ..Default::default()
        };
        let classifier = NsfwImageClassifier::try_load(config);
        assert!(classifier.is_none());
    }

    #[test]
    fn lazy_classifier_default() {
        let lazy = LazyNsfwClassifier::with_defaults();
        assert!(!lazy.is_loaded());
        assert!(!lazy.load_attempted());
    }

    #[test]
    fn lazy_classifier_handles_missing_model() {
        let mut lazy = LazyNsfwClassifier::new(NsfwImageConfig {
            model_path: "nonexistent/model.onnx".to_string(),
            ..Default::default()
        });

        // First attempt
        assert!(!lazy.ensure_loaded());
        assert!(lazy.load_attempted());
        assert!(!lazy.is_loaded());

        // Second attempt should not retry
        assert!(!lazy.ensure_loaded());

        // classify_bytes should return None
        assert!(lazy.classify_bytes(&[]).is_none());
    }

    #[cfg(feature = "ml")]
    #[test]
    fn softmax_works_correctly() {
        let (a, b) = softmax(0.0, 0.0);
        assert!((a - 0.5).abs() < 0.001);
        assert!((b - 0.5).abs() < 0.001);

        let (a, b) = softmax(10.0, 0.0);
        assert!(a > 0.99);
        assert!(b < 0.01);
    }

    #[test]
    fn nsfw_result_serialization() {
        let result = NsfwImageResult {
            sfw_probability: 0.7,
            nsfw_probability: 0.3,
            duration_us: 50000,
            image_dimensions: (640, 480),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: NsfwImageResult = serde_json::from_str(&json).unwrap();

        assert!((result.sfw_probability - deserialized.sfw_probability).abs() < 0.001);
        assert!((result.nsfw_probability - deserialized.nsfw_probability).abs() < 0.001);
        assert_eq!(result.duration_us, deserialized.duration_us);
        assert_eq!(result.image_dimensions, deserialized.image_dimensions);
    }
}

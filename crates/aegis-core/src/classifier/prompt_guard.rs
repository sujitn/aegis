//! Prompt Guard ML classifier (Tier 2).
//!
//! Uses Meta's Prompt Guard model via ONNX for detecting jailbreaks and
//! prompt injection attacks. Designed to run in <50ms on CPU.

#[cfg(feature = "ml")]
use std::path::Path;
#[cfg(feature = "ml")]
use std::time::Instant;

use super::{Category, CategoryMatch, ClassificationResult};

/// Result of Prompt Guard classification.
#[derive(Debug, Clone)]
pub struct PromptGuardResult {
    /// Probability that the content is safe (0.0 to 1.0).
    pub safe_probability: f32,
    /// Probability that the content is unsafe/jailbreak (0.0 to 1.0).
    pub unsafe_probability: f32,
    /// Classification duration in microseconds.
    pub duration_us: u64,
}

impl PromptGuardResult {
    /// Returns true if the content is classified as unsafe (probability > threshold).
    pub fn is_unsafe(&self, threshold: f32) -> bool {
        self.unsafe_probability > threshold
    }

    /// Converts to a ClassificationResult if unsafe.
    pub fn to_classification_result(&self, threshold: f32) -> ClassificationResult {
        if self.is_unsafe(threshold) {
            ClassificationResult::with_matches(
                vec![CategoryMatch::new(
                    Category::Jailbreak,
                    self.unsafe_probability,
                    Some("prompt_guard_ml".to_string()),
                )],
                self.duration_us,
            )
        } else {
            ClassificationResult::safe(self.duration_us)
        }
    }
}

/// Error types for Prompt Guard classifier.
#[derive(Debug, thiserror::Error)]
pub enum PromptGuardError {
    /// Model file not found.
    #[error("Model file not found: {0}")]
    ModelNotFound(String),

    /// Tokenizer file not found.
    #[error("Tokenizer file not found: {0}")]
    TokenizerNotFound(String),

    /// ONNX runtime error.
    #[error("ONNX runtime error: {0}")]
    #[cfg(feature = "ml")]
    OrtError(#[from] ort::Error),

    /// Tokenizer error.
    #[error("Tokenizer error: {0}")]
    #[cfg(feature = "ml")]
    TokenizerError(String),

    /// Inference error.
    #[error("Inference error: {0}")]
    InferenceError(String),

    /// ML feature not enabled.
    #[error("ML feature not enabled - rebuild with --features ml")]
    MlNotEnabled,
}

#[cfg(feature = "ml")]
impl From<tokenizers::Error> for PromptGuardError {
    fn from(e: tokenizers::Error) -> Self {
        PromptGuardError::TokenizerError(e.to_string())
    }
}

/// Configuration for the Prompt Guard classifier.
#[derive(Debug, Clone)]
pub struct PromptGuardConfig {
    /// Path to the ONNX model file.
    pub model_path: String,
    /// Path to the tokenizer.json file.
    pub tokenizer_path: String,
    /// Maximum sequence length (tokens).
    pub max_length: usize,
    /// Threshold for unsafe classification (default: 0.5).
    pub threshold: f32,
}

impl Default for PromptGuardConfig {
    fn default() -> Self {
        Self {
            model_path: "models/prompt_guard.onnx".to_string(),
            tokenizer_path: "models/tokenizer.json".to_string(),
            max_length: 512,
            threshold: 0.5,
        }
    }
}

/// ML-based classifier using Meta's Prompt Guard model.
///
/// This classifier runs ONNX inference to detect jailbreak attempts
/// and prompt injection attacks with high accuracy.
#[cfg(feature = "ml")]
pub struct PromptGuardClassifier {
    session: ort::session::Session,
    tokenizer: tokenizers::Tokenizer,
    config: PromptGuardConfig,
}

#[cfg(feature = "ml")]
impl PromptGuardClassifier {
    /// Creates a new Prompt Guard classifier by loading the ONNX model.
    ///
    /// Returns an error if the model or tokenizer files are not found.
    pub fn new(config: PromptGuardConfig) -> Result<Self, PromptGuardError> {
        use ort::session::{builder::GraphOptimizationLevel, Session};

        // Check if files exist
        if !Path::new(&config.model_path).exists() {
            return Err(PromptGuardError::ModelNotFound(config.model_path.clone()));
        }
        if !Path::new(&config.tokenizer_path).exists() {
            return Err(PromptGuardError::TokenizerNotFound(
                config.tokenizer_path.clone(),
            ));
        }

        // Load ONNX model with optimizations
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(1)?
            .commit_from_file(&config.model_path)?;

        // Load tokenizer
        let tokenizer = tokenizers::Tokenizer::from_file(&config.tokenizer_path)?;

        Ok(Self {
            session,
            tokenizer,
            config,
        })
    }

    /// Loads the classifier from default paths.
    pub fn load_default() -> Result<Self, PromptGuardError> {
        Self::new(PromptGuardConfig::default())
    }

    /// Attempts to load the classifier, returning None if files don't exist.
    ///
    /// This is the preferred way to initialize when graceful fallback is desired.
    pub fn try_load(config: PromptGuardConfig) -> Option<Self> {
        Self::new(config).ok()
    }

    /// Classifies the given text and returns safe/unsafe probabilities.
    pub fn classify(&mut self, text: &str) -> Result<PromptGuardResult, PromptGuardError> {
        use ort::value::Tensor;

        let start = Instant::now();

        // Tokenize input
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| PromptGuardError::TokenizerError(e.to_string()))?;

        // Prepare input tensors
        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();

        let seq_len = input_ids.len().min(self.config.max_length);
        let input_ids = input_ids[..seq_len].to_vec();
        let attention_mask = attention_mask[..seq_len].to_vec();

        // Create ONNX tensors with shape [1, seq_len]
        let input_ids_tensor = Tensor::from_array(([1, seq_len], input_ids.into_boxed_slice()))?;
        let attention_mask_tensor =
            Tensor::from_array(([1, seq_len], attention_mask.into_boxed_slice()))?;

        // Run inference - pass inputs in the order expected by the model
        let outputs = self.session.run(ort::inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => attention_mask_tensor
        ])?;

        // Extract logits from output (first output tensor)
        let logits_tensor = outputs["logits"].try_extract_tensor::<f32>().map_err(|e| {
            PromptGuardError::InferenceError(format!("Failed to extract logits: {}", e))
        })?;

        // Get shape and data
        let shape = logits_tensor.0;
        let logits_data = logits_tensor.1;

        // Verify shape is [1, 2]
        let dims: Vec<_> = shape.iter().collect();
        if dims.len() != 2 || *dims[0] != 1 || *dims[1] != 2 {
            return Err(PromptGuardError::InferenceError(format!(
                "Unexpected output shape: {:?}",
                dims
            )));
        }

        // Apply softmax to get probabilities
        // Output shape is [1, 2] where index 0 = safe, index 1 = unsafe
        let logit_safe = logits_data[0];
        let logit_unsafe = logits_data[1];

        let (safe_prob, unsafe_prob) = softmax(logit_safe, logit_unsafe);

        let duration_us = start.elapsed().as_micros() as u64;

        Ok(PromptGuardResult {
            safe_probability: safe_prob,
            unsafe_probability: unsafe_prob,
            duration_us,
        })
    }

    /// Classifies text and returns a ClassificationResult.
    ///
    /// Uses the configured threshold to determine if content should be blocked.
    pub fn classify_to_result(
        &mut self,
        text: &str,
    ) -> Result<ClassificationResult, PromptGuardError> {
        let result = self.classify(text)?;
        Ok(result.to_classification_result(self.config.threshold))
    }

    /// Returns the configured threshold.
    pub fn threshold(&self) -> f32 {
        self.config.threshold
    }

    /// Sets a new threshold.
    pub fn set_threshold(&mut self, threshold: f32) {
        self.config.threshold = threshold.clamp(0.0, 1.0);
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

/// Stub classifier when ML feature is not enabled.
///
/// Always returns an error indicating ML is not available.
#[cfg(not(feature = "ml"))]
pub struct PromptGuardClassifier {
    _config: PromptGuardConfig,
}

#[cfg(not(feature = "ml"))]
impl PromptGuardClassifier {
    /// Creates a stub classifier (ML feature not enabled).
    pub fn new(_config: PromptGuardConfig) -> Result<Self, PromptGuardError> {
        Err(PromptGuardError::MlNotEnabled)
    }

    /// Loads the classifier from default paths (ML feature not enabled).
    pub fn load_default() -> Result<Self, PromptGuardError> {
        Err(PromptGuardError::MlNotEnabled)
    }

    /// Attempts to load the classifier (always returns None when ML is disabled).
    pub fn try_load(_config: PromptGuardConfig) -> Option<Self> {
        None
    }

    /// Classifies text (ML feature not enabled).
    pub fn classify(&mut self, _text: &str) -> Result<PromptGuardResult, PromptGuardError> {
        Err(PromptGuardError::MlNotEnabled)
    }

    /// Classifies text to result (ML feature not enabled).
    pub fn classify_to_result(
        &mut self,
        _text: &str,
    ) -> Result<ClassificationResult, PromptGuardError> {
        Err(PromptGuardError::MlNotEnabled)
    }

    /// Returns the configured threshold.
    pub fn threshold(&self) -> f32 {
        0.5
    }

    /// Sets a new threshold.
    pub fn set_threshold(&mut self, _threshold: f32) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_guard_result_is_unsafe() {
        let result = PromptGuardResult {
            safe_probability: 0.2,
            unsafe_probability: 0.8,
            duration_us: 100,
        };
        assert!(result.is_unsafe(0.5));
        assert!(result.is_unsafe(0.7));
        assert!(!result.is_unsafe(0.9));
    }

    #[test]
    fn prompt_guard_result_to_classification_result_blocked() {
        let result = PromptGuardResult {
            safe_probability: 0.1,
            unsafe_probability: 0.9,
            duration_us: 100,
        };
        let classification = result.to_classification_result(0.5);
        assert!(classification.should_block);
        assert_eq!(classification.matches.len(), 1);
        assert_eq!(classification.matches[0].category, Category::Jailbreak);
        assert_eq!(classification.matches[0].confidence, 0.9);
    }

    #[test]
    fn prompt_guard_result_to_classification_result_safe() {
        let result = PromptGuardResult {
            safe_probability: 0.9,
            unsafe_probability: 0.1,
            duration_us: 100,
        };
        let classification = result.to_classification_result(0.5);
        assert!(!classification.should_block);
        assert!(classification.matches.is_empty());
    }

    #[test]
    fn config_default_values() {
        let config = PromptGuardConfig::default();
        assert_eq!(config.max_length, 512);
        assert_eq!(config.threshold, 0.5);
    }

    #[test]
    fn try_load_returns_none_when_model_missing() {
        let config = PromptGuardConfig {
            model_path: "nonexistent/model.onnx".to_string(),
            tokenizer_path: "nonexistent/tokenizer.json".to_string(),
            ..Default::default()
        };
        let classifier = PromptGuardClassifier::try_load(config);
        assert!(classifier.is_none());
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
}

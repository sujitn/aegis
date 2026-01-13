//! Content classification for AI safety.
//!
//! This module provides classification functionality to detect potentially
//! harmful content in LLM interactions.
//!
//! ## Classification Tiers
//!
//! - **Tier 1 (Keyword)**: Fast regex-based matching (<1ms). Catches obvious violations.
//! - **Tier 2 (ML)**: Prompt Guard ONNX model (<50ms). Catches sophisticated attacks.
//!
//! ## Tiered Pipeline
//!
//! The [`TieredClassifier`] orchestrates both tiers with short-circuit optimization:
//! 1. Keywords checked first
//! 2. High-confidence matches skip ML (short-circuit)
//! 3. Otherwise, ML runs and results are merged
//!
//! Typical latency: <25ms (keyword-only: <1ms, with ML: <50ms)

mod category;
mod keyword;
mod prompt_guard;
mod tiered;

pub use category::{Category, CategoryMatch, ClassificationResult, ClassificationTier};
pub use keyword::KeywordClassifier;
pub use prompt_guard::{
    PromptGuardClassifier, PromptGuardConfig, PromptGuardError, PromptGuardResult,
};
pub use tiered::{ClassificationStats, SafetyClassifier, TieredClassifier, TieredClassifierConfig};

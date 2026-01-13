//! Content classification for AI safety.
//!
//! This module provides classification functionality to detect potentially
//! harmful content in LLM interactions.
//!
//! ## Classification Tiers
//!
//! - **Tier 1 (Keyword)**: Fast regex-based matching (<1ms). Catches obvious violations.
//! - **Tier 2 (ML)**: Prompt Guard ONNX model (<50ms). Catches sophisticated attacks.

mod category;
mod keyword;
mod prompt_guard;

pub use category::{Category, CategoryMatch, ClassificationResult};
pub use keyword::KeywordClassifier;
pub use prompt_guard::{
    PromptGuardClassifier, PromptGuardConfig, PromptGuardError, PromptGuardResult,
};

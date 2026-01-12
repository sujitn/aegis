//! Content classification for AI safety.
//!
//! This module provides classification functionality to detect potentially
//! harmful content in LLM interactions.

mod category;
mod keyword;

pub use category::{Category, CategoryMatch, ClassificationResult};
pub use keyword::KeywordClassifier;

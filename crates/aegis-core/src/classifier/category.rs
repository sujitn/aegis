//! Safety categories for content classification.

use serde::{Deserialize, Serialize};

/// Safety categories that content can be classified into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Content promoting or describing violence.
    Violence,
    /// Content related to self-harm or suicide.
    SelfHarm,
    /// Adult or sexually explicit content.
    Adult,
    /// Attempts to bypass AI safety measures.
    Jailbreak,
    /// Hate speech or discrimination.
    Hate,
    /// Content promoting illegal activities.
    Illegal,
}

impl Category {
    /// Returns all available categories.
    pub fn all() -> &'static [Category] {
        &[
            Category::Violence,
            Category::SelfHarm,
            Category::Adult,
            Category::Jailbreak,
            Category::Hate,
            Category::Illegal,
        ]
    }

    /// Returns a human-readable name for this category.
    pub fn name(&self) -> &'static str {
        match self {
            Category::Violence => "Violence",
            Category::SelfHarm => "Self-Harm",
            Category::Adult => "Adult",
            Category::Jailbreak => "Jailbreak",
            Category::Hate => "Hate",
            Category::Illegal => "Illegal",
        }
    }
}

/// A single match from classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryMatch {
    /// The matched category.
    pub category: Category,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// The pattern or keyword that matched (if available).
    pub matched_pattern: Option<String>,
}

impl CategoryMatch {
    /// Creates a new category match.
    pub fn new(category: Category, confidence: f32, matched_pattern: Option<String>) -> Self {
        Self {
            category,
            confidence: confidence.clamp(0.0, 1.0),
            matched_pattern,
        }
    }
}

/// Result of classifying content.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// All category matches found.
    pub matches: Vec<CategoryMatch>,
    /// Whether the content should be blocked.
    pub should_block: bool,
    /// Classification duration in microseconds.
    pub duration_us: u64,
}

impl ClassificationResult {
    /// Creates an empty (safe) classification result.
    pub fn safe(duration_us: u64) -> Self {
        Self {
            matches: Vec::new(),
            should_block: false,
            duration_us,
        }
    }

    /// Creates a classification result with matches.
    pub fn with_matches(matches: Vec<CategoryMatch>, duration_us: u64) -> Self {
        let should_block = !matches.is_empty();
        Self {
            matches,
            should_block,
            duration_us,
        }
    }

    /// Returns the highest confidence match, if any.
    pub fn highest_confidence(&self) -> Option<&CategoryMatch> {
        self.matches
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
    }

    /// Returns true if any category matched.
    pub fn has_matches(&self) -> bool {
        !self.matches.is_empty()
    }

    /// Returns matches for a specific category.
    pub fn matches_for(&self, category: Category) -> Vec<&CategoryMatch> {
        self.matches
            .iter()
            .filter(|m| m.category == category)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_all_returns_all_variants() {
        let all = Category::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn category_match_clamps_confidence() {
        let m = CategoryMatch::new(Category::Violence, 1.5, None);
        assert_eq!(m.confidence, 1.0);

        let m = CategoryMatch::new(Category::Violence, -0.5, None);
        assert_eq!(m.confidence, 0.0);
    }

    #[test]
    fn classification_result_safe() {
        let result = ClassificationResult::safe(100);
        assert!(!result.should_block);
        assert!(!result.has_matches());
        assert_eq!(result.duration_us, 100);
    }

    #[test]
    fn classification_result_with_matches() {
        let matches = vec![
            CategoryMatch::new(Category::Violence, 0.9, Some("kill".to_string())),
            CategoryMatch::new(Category::Hate, 0.7, None),
        ];
        let result = ClassificationResult::with_matches(matches, 50);
        assert!(result.should_block);
        assert!(result.has_matches());
        assert_eq!(result.matches.len(), 2);
    }

    #[test]
    fn highest_confidence_returns_max() {
        let matches = vec![
            CategoryMatch::new(Category::Violence, 0.5, None),
            CategoryMatch::new(Category::Hate, 0.9, None),
            CategoryMatch::new(Category::Adult, 0.3, None),
        ];
        let result = ClassificationResult::with_matches(matches, 50);
        let highest = result.highest_confidence().unwrap();
        assert_eq!(highest.category, Category::Hate);
        assert_eq!(highest.confidence, 0.9);
    }
}

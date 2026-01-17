//! Sentiment and emotional analysis for parental review flagging.
//!
//! This module provides lexicon-based sentiment analysis to identify content
//! that may indicate emotional distress or concerning topics. Unlike the
//! blocking classifiers (Tier 1/2), this flags content for parental review.
//!
//! ## Sentiment Categories
//!
//! - **Distress**: Sadness, hopelessness, anxiety, loneliness
//! - **CrisisIndicator**: Self-harm adjacent language, suicidal ideation
//! - **Bullying**: Peer conflict, harassment discussion
//! - **NegativeSentiment**: Sustained negative patterns, anger
//!
//! ## Performance Target
//!
//! <10ms latency for analysis.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Sentiment flag categories for parental review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SentimentFlag {
    /// Emotional distress: sadness, hopelessness, anxiety, loneliness.
    Distress,
    /// Crisis indicators: self-harm adjacent, suicidal ideation.
    CrisisIndicator,
    /// Bullying: peer conflict, harassment discussion.
    Bullying,
    /// Sustained negative sentiment patterns.
    NegativeSentiment,
}

impl SentimentFlag {
    /// Returns all sentiment flag categories.
    pub fn all() -> &'static [SentimentFlag] {
        &[
            SentimentFlag::Distress,
            SentimentFlag::CrisisIndicator,
            SentimentFlag::Bullying,
            SentimentFlag::NegativeSentiment,
        ]
    }

    /// Returns a human-readable name for this flag.
    pub fn name(&self) -> &'static str {
        match self {
            SentimentFlag::Distress => "Emotional Distress",
            SentimentFlag::CrisisIndicator => "Crisis Indicator",
            SentimentFlag::Bullying => "Bullying",
            SentimentFlag::NegativeSentiment => "Negative Sentiment",
        }
    }

    /// Returns a description of what this flag indicates.
    pub fn description(&self) -> &'static str {
        match self {
            SentimentFlag::Distress => {
                "Content expressing sadness, hopelessness, anxiety, or loneliness"
            }
            SentimentFlag::CrisisIndicator => {
                "Content that may indicate self-harm ideation or crisis"
            }
            SentimentFlag::Bullying => "Content discussing bullying, harassment, or peer conflict",
            SentimentFlag::NegativeSentiment => "Content with sustained negative emotional tone",
        }
    }
}

/// A single sentiment match from analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentimentMatch {
    /// The matched sentiment flag.
    pub flag: SentimentFlag,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// Phrases or words that triggered this match.
    pub matched_phrases: Vec<String>,
}

impl SentimentMatch {
    /// Creates a new sentiment match.
    pub fn new(flag: SentimentFlag, confidence: f32, matched_phrases: Vec<String>) -> Self {
        Self {
            flag,
            confidence: confidence.clamp(0.0, 1.0),
            matched_phrases,
        }
    }
}

/// Result of sentiment analysis.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SentimentResult {
    /// All sentiment flags detected.
    pub flags: Vec<SentimentMatch>,
    /// Overall sentiment score (-1.0 negative to 1.0 positive).
    pub overall_sentiment: f32,
    /// Analysis duration in microseconds.
    pub duration_us: u64,
}

impl SentimentResult {
    /// Creates an empty (neutral) sentiment result.
    pub fn neutral(duration_us: u64) -> Self {
        Self {
            flags: Vec::new(),
            overall_sentiment: 0.0,
            duration_us,
        }
    }

    /// Returns true if any flags were detected.
    pub fn has_flags(&self) -> bool {
        !self.flags.is_empty()
    }

    /// Returns the highest confidence flag, if any.
    pub fn highest_confidence(&self) -> Option<&SentimentMatch> {
        self.flags
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
    }

    /// Returns flags for a specific category.
    pub fn flags_for(&self, flag: SentimentFlag) -> Vec<&SentimentMatch> {
        self.flags.iter().filter(|m| m.flag == flag).collect()
    }
}

/// Configuration for sentiment analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentConfig {
    /// Whether sentiment analysis is enabled.
    pub enabled: bool,
    /// Minimum confidence threshold to flag (0.0-1.0).
    pub threshold: f32,
    /// Which sentiment categories to detect.
    pub enabled_flags: HashSet<SentimentFlag>,
    /// Whether to notify parent when content is flagged.
    pub notify_on_flag: bool,
}

impl Default for SentimentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 0.6,
            enabled_flags: SentimentFlag::all().iter().copied().collect(),
            notify_on_flag: true,
        }
    }
}

/// Word entry in the sentiment lexicon.
#[derive(Debug, Clone)]
struct LexiconEntry {
    /// Valence score (-1.0 to 1.0).
    valence: f32,
    /// Associated sentiment flags.
    flags: Vec<SentimentFlag>,
    /// Weight/importance of this word.
    weight: f32,
}

/// Lexicon-based sentiment analyzer.
///
/// Uses word lists and phrase patterns to detect emotional content.
/// Inspired by VADER sentiment analysis with domain-specific extensions.
pub struct SentimentAnalyzer {
    /// Word-level sentiment lexicon.
    lexicon: HashMap<String, LexiconEntry>,
    /// Multi-word phrase patterns per flag.
    phrase_patterns: HashMap<SentimentFlag, Vec<PhrasePattern>>,
    /// Intensifier words that boost sentiment.
    intensifiers: HashMap<String, f32>,
    /// Negation words that flip sentiment.
    negations: HashSet<String>,
    /// Configuration.
    config: SentimentConfig,
}

/// A phrase pattern for matching.
#[derive(Debug, Clone)]
struct PhrasePattern {
    /// Words in the phrase (lowercase).
    words: Vec<String>,
    /// Confidence boost when matched.
    confidence: f32,
}

impl SentimentAnalyzer {
    /// Creates a new sentiment analyzer with default lexicons.
    pub fn new(config: SentimentConfig) -> Self {
        let mut analyzer = Self {
            lexicon: HashMap::new(),
            phrase_patterns: HashMap::new(),
            intensifiers: HashMap::new(),
            negations: HashSet::new(),
            config,
        };
        analyzer.load_default_lexicons();
        analyzer
    }

    /// Creates an analyzer with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(SentimentConfig::default())
    }

    /// Analyzes text for sentiment and emotional indicators.
    pub fn analyze(&self, text: &str) -> SentimentResult {
        if !self.config.enabled {
            return SentimentResult::neutral(0);
        }

        let start = Instant::now();
        let text_lower = text.to_lowercase();
        let words: Vec<&str> = text_lower.split_whitespace().collect();

        // Phase 1: Calculate overall sentiment
        let overall_sentiment = self.calculate_overall_sentiment(&words);

        // Phase 2: Detect specific flags
        let mut flags = Vec::new();

        // Check phrase patterns first (higher accuracy)
        for flag in SentimentFlag::all() {
            if !self.config.enabled_flags.contains(flag) {
                continue;
            }

            if let Some(patterns) = self.phrase_patterns.get(flag) {
                let mut matched_phrases = Vec::new();
                let mut max_confidence = 0.0f32;

                for pattern in patterns {
                    if self.matches_phrase(&text_lower, pattern) {
                        matched_phrases.push(pattern.words.join(" "));
                        max_confidence = max_confidence.max(pattern.confidence);
                    }
                }

                if !matched_phrases.is_empty() && max_confidence >= self.config.threshold {
                    flags.push(SentimentMatch::new(*flag, max_confidence, matched_phrases));
                }
            }
        }

        // Check word-level patterns for flags not yet matched
        let matched_flag_types: HashSet<_> = flags.iter().map(|f| f.flag).collect();

        for flag in SentimentFlag::all() {
            if !self.config.enabled_flags.contains(flag) || matched_flag_types.contains(flag) {
                continue;
            }

            let (confidence, matched_words) = self.calculate_flag_score(&words, *flag);
            if confidence >= self.config.threshold && !matched_words.is_empty() {
                flags.push(SentimentMatch::new(*flag, confidence, matched_words));
            }
        }

        // Check for NegativeSentiment based on overall score
        if self
            .config
            .enabled_flags
            .contains(&SentimentFlag::NegativeSentiment)
            && !matched_flag_types.contains(&SentimentFlag::NegativeSentiment)
        {
            // Flag if overall sentiment is strongly negative
            if overall_sentiment < -0.5 {
                let confidence = (-overall_sentiment).min(1.0);
                if confidence >= self.config.threshold {
                    flags.push(SentimentMatch::new(
                        SentimentFlag::NegativeSentiment,
                        confidence,
                        vec!["overall negative tone".to_string()],
                    ));
                }
            }
        }

        let duration_us = start.elapsed().as_micros() as u64;

        SentimentResult {
            flags,
            overall_sentiment,
            duration_us,
        }
    }

    /// Calculates the overall sentiment score.
    fn calculate_overall_sentiment(&self, words: &[&str]) -> f32 {
        if words.is_empty() {
            return 0.0;
        }

        let mut total_score = 0.0;
        let mut total_weight = 0.0;
        let mut negation_active = false;
        let mut negation_distance = 0;
        let mut pending_intensifier = 1.0f32;

        for word in words.iter() {
            // Check for negation
            if self.negations.contains(*word) {
                negation_active = true;
                negation_distance = 0;
                continue;
            }

            // Check for intensifier (save for next sentiment word)
            if let Some(&boost) = self.intensifiers.get(*word) {
                pending_intensifier = boost;
                continue;
            }

            // Look up word in lexicon
            if let Some(entry) = self.lexicon.get(*word) {
                let mut score = entry.valence * entry.weight * pending_intensifier;

                // Apply negation (within 3 words)
                if negation_active && negation_distance < 3 {
                    score = -score * 0.7; // Dampen and flip negated sentiment
                }

                total_score += score;
                total_weight += entry.weight;

                // Reset intensifier after use
                pending_intensifier = 1.0;
            }

            // Track distance from negation
            if negation_active {
                negation_distance += 1;
                if negation_distance >= 3 {
                    negation_active = false;
                }
            }
        }

        if total_weight > 0.0 {
            (total_score / total_weight).clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }

    /// Checks if text contains a phrase pattern.
    fn matches_phrase(&self, text: &str, pattern: &PhrasePattern) -> bool {
        if pattern.words.is_empty() {
            return false;
        }

        // Simple substring matching for phrases
        let phrase = pattern.words.join(" ");
        text.contains(&phrase)
    }

    /// Calculates score for a specific flag based on word matches.
    fn calculate_flag_score(&self, words: &[&str], flag: SentimentFlag) -> (f32, Vec<String>) {
        let mut matched_words = Vec::new();
        let mut total_weight = 0.0;

        for word in words {
            if let Some(entry) = self.lexicon.get(*word) {
                if entry.flags.contains(&flag) {
                    matched_words.push(word.to_string());
                    total_weight += entry.weight;
                }
            }
        }

        // Normalize score based on number of matches and their weights
        let confidence = if matched_words.is_empty() {
            0.0
        } else {
            // Combine match count and total weight for confidence
            let base = 0.5 + (matched_words.len() as f32 * 0.1);
            let weight_bonus = (total_weight / 5.0).min(0.3);
            (base + weight_bonus).min(1.0)
        };

        (confidence, matched_words)
    }

    /// Loads default lexicons for sentiment analysis.
    fn load_default_lexicons(&mut self) {
        // Load intensifiers
        self.load_intensifiers();

        // Load negations
        self.load_negations();

        // Load word lexicon
        self.load_word_lexicon();

        // Load phrase patterns
        self.load_phrase_patterns();
    }

    fn load_intensifiers(&mut self) {
        let intensifiers = [
            ("very", 1.3),
            ("really", 1.3),
            ("extremely", 1.5),
            ("absolutely", 1.5),
            ("totally", 1.3),
            ("so", 1.2),
            ("incredibly", 1.4),
            ("terribly", 1.4),
            ("deeply", 1.3),
            ("always", 1.2),
            ("never", 1.2),
            ("completely", 1.4),
        ];

        for (word, boost) in intensifiers {
            self.intensifiers.insert(word.to_string(), boost);
        }
    }

    fn load_negations(&mut self) {
        let negations = [
            "not",
            "no",
            "never",
            "none",
            "nobody",
            "nothing",
            "neither",
            "nowhere",
            "cannot",
            "can't",
            "don't",
            "doesn't",
            "didn't",
            "won't",
            "wouldn't",
            "couldn't",
            "shouldn't",
            "isn't",
            "aren't",
            "wasn't",
            "weren't",
            "haven't",
            "hasn't",
            "hadn't",
        ];

        for word in negations {
            self.negations.insert(word.to_string());
        }
    }

    fn load_word_lexicon(&mut self) {
        // Distress words
        let distress_words = [
            ("sad", -0.7, 1.0),
            ("depressed", -0.9, 1.2),
            ("lonely", -0.8, 1.1),
            ("alone", -0.6, 1.0),
            ("hopeless", -0.9, 1.2),
            ("worthless", -0.9, 1.2),
            ("helpless", -0.8, 1.1),
            ("miserable", -0.8, 1.1),
            ("empty", -0.6, 0.9),
            ("anxious", -0.7, 1.0),
            ("worried", -0.5, 0.8),
            ("scared", -0.6, 0.9),
            ("afraid", -0.6, 0.9),
            ("terrified", -0.8, 1.1),
            ("crying", -0.6, 1.0),
            ("tears", -0.5, 0.9),
            ("heartbroken", -0.8, 1.1),
            ("devastated", -0.9, 1.2),
            ("exhausted", -0.5, 0.8),
            ("overwhelmed", -0.7, 1.0),
            ("struggling", -0.6, 0.9),
            ("suffering", -0.8, 1.1),
            ("pain", -0.6, 0.9),
            ("hurt", -0.6, 0.9),
            ("broken", -0.7, 1.0),
            ("lost", -0.5, 0.8),
            ("confused", -0.4, 0.7),
            ("trapped", -0.7, 1.0),
            ("stuck", -0.5, 0.8),
            ("failure", -0.7, 1.0),
            ("failed", -0.6, 0.9),
            ("rejected", -0.7, 1.0),
            ("abandoned", -0.8, 1.1),
            ("ignored", -0.6, 0.9),
            ("invisible", -0.6, 0.9),
            ("unwanted", -0.8, 1.1),
            ("unloved", -0.8, 1.1),
        ];

        for (word, valence, weight) in distress_words {
            self.lexicon.insert(
                word.to_string(),
                LexiconEntry {
                    valence,
                    flags: vec![SentimentFlag::Distress],
                    weight,
                },
            );
        }

        // Crisis indicator words (self-harm adjacent)
        let crisis_words = [
            ("suicide", -1.0, 1.5),
            ("suicidal", -1.0, 1.5),
            ("die", -0.8, 1.2),
            ("dying", -0.8, 1.2),
            ("death", -0.7, 1.0),
            ("dead", -0.7, 1.0),
            ("kill", -0.9, 1.3),
            ("cutting", -0.8, 1.2),
            ("harm", -0.7, 1.0),
            ("ending", -0.5, 0.9),
            ("disappear", -0.6, 1.0),
            ("goodbye", -0.4, 0.8),
            ("burden", -0.7, 1.1),
            ("pills", -0.5, 0.9),
            ("overdose", -0.9, 1.4),
        ];

        for (word, valence, weight) in crisis_words {
            self.lexicon.insert(
                word.to_string(),
                LexiconEntry {
                    valence,
                    flags: vec![SentimentFlag::CrisisIndicator],
                    weight,
                },
            );
        }

        // Bullying words
        let bullying_words = [
            ("bully", -0.8, 1.2),
            ("bullied", -0.8, 1.2),
            ("bullying", -0.8, 1.2),
            ("mean", -0.5, 0.8),
            ("cruel", -0.7, 1.0),
            ("harass", -0.8, 1.1),
            ("harassed", -0.8, 1.1),
            ("harassment", -0.8, 1.1),
            ("teasing", -0.5, 0.8),
            ("teased", -0.5, 0.8),
            ("mocking", -0.6, 0.9),
            ("mocked", -0.6, 0.9),
            ("laughing", -0.3, 0.6),
            ("excluded", -0.7, 1.0),
            ("outcast", -0.7, 1.0),
            ("rumors", -0.6, 0.9),
            ("gossip", -0.5, 0.8),
            ("spreading", -0.4, 0.7),
            ("embarrassed", -0.6, 0.9),
            ("humiliated", -0.8, 1.1),
            ("threatened", -0.8, 1.1),
            ("intimidated", -0.7, 1.0),
            ("picked", -0.4, 0.7), // "picked on"
        ];

        for (word, valence, weight) in bullying_words {
            self.lexicon.insert(
                word.to_string(),
                LexiconEntry {
                    valence,
                    flags: vec![SentimentFlag::Bullying],
                    weight,
                },
            );
        }

        // General negative sentiment words
        let negative_words = [
            ("hate", -0.8, 1.0),
            ("angry", -0.7, 0.9),
            ("furious", -0.9, 1.1),
            ("annoyed", -0.5, 0.7),
            ("frustrated", -0.6, 0.8),
            ("irritated", -0.5, 0.7),
            ("mad", -0.6, 0.8),
            ("upset", -0.6, 0.8),
            ("terrible", -0.7, 0.9),
            ("awful", -0.7, 0.9),
            ("horrible", -0.8, 1.0),
            ("worst", -0.8, 1.0),
            ("bad", -0.5, 0.7),
            ("stupid", -0.5, 0.7),
            ("dumb", -0.5, 0.7),
            ("idiot", -0.6, 0.8),
            ("ugly", -0.6, 0.8),
            ("fat", -0.5, 0.7),
            ("loser", -0.7, 0.9),
            ("useless", -0.7, 0.9),
            ("pathetic", -0.7, 0.9),
            ("disgusting", -0.7, 0.9),
            ("sick", -0.4, 0.6),
            ("tired", -0.3, 0.5),
        ];

        for (word, valence, weight) in negative_words {
            // Only add if not already present (don't overwrite more specific entries)
            self.lexicon
                .entry(word.to_string())
                .or_insert(LexiconEntry {
                    valence,
                    flags: vec![SentimentFlag::NegativeSentiment],
                    weight,
                });
        }

        // Positive words (for overall sentiment calculation)
        let positive_words = [
            ("happy", 0.8, 1.0),
            ("joy", 0.9, 1.1),
            ("love", 0.8, 1.0),
            ("great", 0.7, 0.9),
            ("good", 0.6, 0.8),
            ("wonderful", 0.8, 1.0),
            ("amazing", 0.8, 1.0),
            ("awesome", 0.8, 1.0),
            ("excellent", 0.8, 1.0),
            ("fantastic", 0.8, 1.0),
            ("beautiful", 0.7, 0.9),
            ("nice", 0.5, 0.7),
            ("kind", 0.6, 0.8),
            ("caring", 0.6, 0.8),
            ("helpful", 0.6, 0.8),
            ("friend", 0.5, 0.7),
            ("friends", 0.5, 0.7),
            ("fun", 0.6, 0.8),
            ("excited", 0.7, 0.9),
            ("proud", 0.7, 0.9),
            ("confident", 0.6, 0.8),
            ("calm", 0.5, 0.7),
            ("peaceful", 0.6, 0.8),
            ("grateful", 0.7, 0.9),
            ("thankful", 0.7, 0.9),
            ("hopeful", 0.6, 0.8),
            ("optimistic", 0.6, 0.8),
        ];

        for (word, valence, weight) in positive_words {
            self.lexicon
                .entry(word.to_string())
                .or_insert(LexiconEntry {
                    valence,
                    flags: vec![],
                    weight,
                });
        }
    }

    fn load_phrase_patterns(&mut self) {
        // Distress phrases
        let distress_phrases = [
            ("i feel so alone", 0.85),
            ("nobody cares", 0.80),
            ("nobody understands", 0.75),
            ("no one understands", 0.75),
            ("i feel empty", 0.80),
            ("i feel worthless", 0.85),
            ("i feel hopeless", 0.85),
            ("i feel like a failure", 0.80),
            ("i hate myself", 0.85),
            ("i'm so tired of", 0.70),
            ("i can't take it anymore", 0.85),
            ("i can't do this anymore", 0.85),
            ("what's the point", 0.75),
            ("why even try", 0.75),
            ("nothing matters", 0.80),
            ("i feel trapped", 0.80),
            ("i feel stuck", 0.75),
            ("no way out", 0.80),
            ("i'm a burden", 0.85),
            ("everyone would be better", 0.85),
            ("no one would miss me", 0.90),
            ("i don't belong", 0.80),
            ("i feel invisible", 0.75),
        ];

        let distress_patterns: Vec<PhrasePattern> = distress_phrases
            .iter()
            .map(|(phrase, confidence)| PhrasePattern {
                words: phrase.split_whitespace().map(String::from).collect(),
                confidence: *confidence,
            })
            .collect();

        self.phrase_patterns
            .insert(SentimentFlag::Distress, distress_patterns);

        // Crisis indicator phrases
        let crisis_phrases = [
            ("want to die", 0.95),
            ("want to end it", 0.95),
            ("end my life", 0.95),
            ("kill myself", 0.95),
            ("don't want to be here", 0.85),
            ("don't want to exist", 0.90),
            ("better off dead", 0.95),
            ("better off without me", 0.90),
            ("hurt myself", 0.90),
            ("harm myself", 0.90),
            ("cutting myself", 0.90),
            ("not worth living", 0.90),
            ("life isn't worth", 0.85),
            ("goodbye forever", 0.85),
            ("final goodbye", 0.85),
            ("ending it all", 0.95),
            ("no reason to live", 0.90),
            ("give up on life", 0.85),
        ];

        let crisis_patterns: Vec<PhrasePattern> = crisis_phrases
            .iter()
            .map(|(phrase, confidence)| PhrasePattern {
                words: phrase.split_whitespace().map(String::from).collect(),
                confidence: *confidence,
            })
            .collect();

        self.phrase_patterns
            .insert(SentimentFlag::CrisisIndicator, crisis_patterns);

        // Bullying phrases
        let bullying_phrases = [
            ("they make fun of me", 0.85),
            ("everyone laughs at me", 0.85),
            ("they call me names", 0.85),
            ("they won't let me", 0.75),
            ("they exclude me", 0.80),
            ("no one wants to", 0.75),
            ("they spread rumors", 0.85),
            ("they're saying things", 0.70),
            ("they pick on me", 0.85),
            ("they bully me", 0.90),
            ("being bullied", 0.90),
            ("getting bullied", 0.90),
            ("they threaten me", 0.90),
            ("they hurt me", 0.85),
            ("they pushed me", 0.85),
            ("they hit me", 0.90),
            ("afraid to go to school", 0.85),
            ("scared of them", 0.80),
        ];

        let bullying_patterns: Vec<PhrasePattern> = bullying_phrases
            .iter()
            .map(|(phrase, confidence)| PhrasePattern {
                words: phrase.split_whitespace().map(String::from).collect(),
                confidence: *confidence,
            })
            .collect();

        self.phrase_patterns
            .insert(SentimentFlag::Bullying, bullying_patterns);
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &SentimentConfig {
        &self.config
    }

    /// Updates the configuration.
    pub fn set_config(&mut self, config: SentimentConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyzer() -> SentimentAnalyzer {
        SentimentAnalyzer::with_defaults()
    }

    #[test]
    fn test_neutral_text() {
        let a = analyzer();
        // Text without emotional words should have near-zero sentiment
        let result = a.analyze("The weather today is fine");
        assert!(!result.has_flags());
        assert_eq!(result.overall_sentiment, 0.0);

        // Text with mild positive word should have low positive sentiment (not flagged)
        let result2 = a.analyze("The weather is nice today");
        assert!(!result2.has_flags());
        // "nice" is a mild positive word, so sentiment should be positive but moderate
        assert!(result2.overall_sentiment > 0.0 && result2.overall_sentiment < 0.8);
    }

    #[test]
    fn test_positive_text() {
        let a = analyzer();
        let result = a.analyze("I am so happy and excited about this wonderful day");
        assert!(result.overall_sentiment > 0.3);
    }

    #[test]
    fn test_distress_detection() {
        let a = analyzer();
        let result = a.analyze("I feel so alone and nobody cares about me");
        assert!(result.has_flags());
        assert!(result
            .flags
            .iter()
            .any(|f| f.flag == SentimentFlag::Distress));
    }

    #[test]
    fn test_crisis_indicator_detection() {
        let a = analyzer();
        let result = a.analyze("I don't want to be here anymore");
        assert!(result.has_flags());
        assert!(result
            .flags
            .iter()
            .any(|f| f.flag == SentimentFlag::CrisisIndicator));
    }

    #[test]
    fn test_bullying_detection() {
        let a = analyzer();
        let result = a.analyze("They make fun of me every day at school");
        assert!(result.has_flags());
        assert!(result
            .flags
            .iter()
            .any(|f| f.flag == SentimentFlag::Bullying));
    }

    #[test]
    fn test_negative_sentiment_detection() {
        let a = analyzer();
        let result =
            a.analyze("Everything is terrible and horrible and I hate this awful situation");
        assert!(result.has_flags());
        assert!(result.overall_sentiment < -0.3);
    }

    #[test]
    fn test_threshold_filtering() {
        let config = SentimentConfig {
            threshold: 0.95, // Very high threshold
            ..Default::default()
        };
        let a = SentimentAnalyzer::new(config);
        let result = a.analyze("I feel a little sad today");
        // Low confidence matches should be filtered out
        assert!(!result.has_flags() || result.highest_confidence().unwrap().confidence >= 0.95);
    }

    #[test]
    fn test_disabled_flags() {
        let mut enabled = HashSet::new();
        enabled.insert(SentimentFlag::Distress);
        // Only Distress enabled, not CrisisIndicator

        let config = SentimentConfig {
            enabled_flags: enabled,
            ..Default::default()
        };
        let a = SentimentAnalyzer::new(config);
        let result = a.analyze("I want to die");
        // CrisisIndicator should not be flagged since it's disabled
        assert!(!result
            .flags
            .iter()
            .any(|f| f.flag == SentimentFlag::CrisisIndicator));
    }

    #[test]
    fn test_disabled_analyzer() {
        let config = SentimentConfig {
            enabled: false,
            ..Default::default()
        };
        let a = SentimentAnalyzer::new(config);
        let result = a.analyze("I want to die and hurt myself");
        assert!(!result.has_flags());
        assert_eq!(result.duration_us, 0);
    }

    #[test]
    fn test_performance() {
        let a = analyzer();
        let start = std::time::Instant::now();

        for _ in 0..100 {
            a.analyze(
                "This is a test sentence with some emotional content about feeling sad and lonely",
            );
        }

        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() / 100;

        // Should be well under 10ms (10000us) target
        assert!(
            avg_us < 10000,
            "Average analysis time {}us exceeds 10ms target",
            avg_us
        );
    }

    #[test]
    fn test_negation_handling() {
        let a = analyzer();

        // "not sad" should have less negative sentiment than "sad"
        let sad_result = a.analyze("I am sad");
        let not_sad_result = a.analyze("I am not sad");

        assert!(
            not_sad_result.overall_sentiment > sad_result.overall_sentiment,
            "Negation should reduce negative sentiment"
        );
    }

    #[test]
    fn test_intensifier_handling() {
        let a = analyzer();

        // "very sad" should be more negative than "sad"
        let sad_result = a.analyze("I am sad");
        let very_sad_result = a.analyze("I am very sad");

        assert!(
            very_sad_result.overall_sentiment < sad_result.overall_sentiment,
            "Intensifier should increase sentiment magnitude"
        );
    }

    #[test]
    fn test_sentiment_flag_all() {
        assert_eq!(SentimentFlag::all().len(), 4);
    }

    #[test]
    fn test_sentiment_flag_names() {
        assert_eq!(SentimentFlag::Distress.name(), "Emotional Distress");
        assert_eq!(SentimentFlag::CrisisIndicator.name(), "Crisis Indicator");
        assert_eq!(SentimentFlag::Bullying.name(), "Bullying");
        assert_eq!(
            SentimentFlag::NegativeSentiment.name(),
            "Negative Sentiment"
        );
    }
}

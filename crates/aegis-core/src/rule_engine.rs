//! Rule engine for evaluating classifications against time and content rules (F007).
//!
//! This module orchestrates time rules (F005) and content rules (F006) to produce
//! a unified filtering decision.
//!
//! ## Evaluation Order
//!
//! 1. Time rules checked first - if blocked, return immediately
//! 2. Content rules checked against classification matches
//! 3. Default allow if no rules match
//!
//! The first matching rule determines the action, with time rules taking precedence.

use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};

use crate::classifier::ClassificationResult;
use crate::content_rules::{ContentAction, ContentRuleResult, ContentRuleSet};
use crate::time_rules::{TimeOfDay, TimeRule, TimeRuleSet, Weekday};

/// Action to take based on rule evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleAction {
    /// Allow the request.
    #[default]
    Allow,
    /// Warn but allow the request.
    Warn,
    /// Block the request.
    Block,
}

impl RuleAction {
    /// Returns a human-readable name for this action.
    pub fn name(&self) -> &'static str {
        match self {
            RuleAction::Allow => "Allow",
            RuleAction::Warn => "Warn",
            RuleAction::Block => "Block",
        }
    }
}

/// Which type of rule triggered the action.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RuleSource {
    /// No rule triggered (default allow).
    #[default]
    None,
    /// A time rule triggered the action.
    TimeRule {
        /// The ID of the time rule that triggered.
        rule_id: String,
        /// The name of the time rule that triggered.
        rule_name: String,
    },
    /// A content rule triggered the action.
    ContentRule(ContentRuleResult),
}

impl RuleSource {
    /// Returns the rule ID if a rule triggered, None otherwise.
    pub fn rule_id(&self) -> Option<&str> {
        match self {
            RuleSource::None => None,
            RuleSource::TimeRule { rule_id, .. } => Some(rule_id),
            RuleSource::ContentRule(result) => Some(&result.rule_id),
        }
    }

    /// Returns the rule name if a rule triggered, None otherwise.
    pub fn rule_name(&self) -> Option<&str> {
        match self {
            RuleSource::None => None,
            RuleSource::TimeRule { rule_name, .. } => Some(rule_name),
            RuleSource::ContentRule(result) => Some(&result.rule_name),
        }
    }

    /// Returns true if a rule triggered.
    pub fn has_rule(&self) -> bool {
        !matches!(self, RuleSource::None)
    }

    /// Returns true if a time rule triggered.
    pub fn is_time_rule(&self) -> bool {
        matches!(self, RuleSource::TimeRule { .. })
    }

    /// Returns true if a content rule triggered.
    pub fn is_content_rule(&self) -> bool {
        matches!(self, RuleSource::ContentRule(_))
    }
}

/// Result of evaluating rules against a classification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleEngineResult {
    /// The action to take.
    pub action: RuleAction,
    /// Which rule triggered the action.
    pub source: RuleSource,
}

impl RuleEngineResult {
    /// Creates an allow result with no rule source.
    pub fn allow() -> Self {
        Self {
            action: RuleAction::Allow,
            source: RuleSource::None,
        }
    }

    /// Creates a block result from a time rule.
    pub fn blocked_by_time(rule: &TimeRule) -> Self {
        Self {
            action: RuleAction::Block,
            source: RuleSource::TimeRule {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
            },
        }
    }

    /// Creates a result from a content rule evaluation.
    pub fn from_content_result(result: ContentRuleResult) -> Self {
        let action = match result.action {
            ContentAction::Block => RuleAction::Block,
            ContentAction::Warn => RuleAction::Warn,
            ContentAction::Allow => RuleAction::Allow,
        };
        Self {
            action,
            source: RuleSource::ContentRule(result),
        }
    }

    /// Returns true if the action is Block.
    pub fn should_block(&self) -> bool {
        self.action == RuleAction::Block
    }

    /// Returns true if the action is Warn.
    pub fn should_warn(&self) -> bool {
        self.action == RuleAction::Warn
    }

    /// Returns true if the action is Allow.
    pub fn should_allow(&self) -> bool {
        self.action == RuleAction::Allow
    }
}

/// Rule engine that evaluates time and content rules.
///
/// The rule engine combines time-based rules (e.g., bedtime, school hours)
/// with content-based rules (e.g., violence blocking) to produce a unified
/// filtering decision.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleEngine {
    /// Time-based rules.
    pub time_rules: TimeRuleSet,
    /// Content-based rules.
    pub content_rules: ContentRuleSet,
}

impl RuleEngine {
    /// Creates an empty rule engine.
    pub fn new() -> Self {
        Self {
            time_rules: TimeRuleSet::new(),
            content_rules: ContentRuleSet::new(),
        }
    }

    /// Creates a rule engine with default rules.
    ///
    /// Time rules: Bedtime school nights (Sun-Thu 9pm-7am), weekends (Fri-Sat 11pm-8am)
    /// Content rules: Family-safe defaults (block violence, self-harm, adult, etc.)
    pub fn with_defaults() -> Self {
        Self {
            time_rules: TimeRuleSet::with_defaults(),
            content_rules: ContentRuleSet::family_safe_defaults(),
        }
    }

    /// Creates a rule engine with permissive content rules.
    pub fn with_permissive_content() -> Self {
        Self {
            time_rules: TimeRuleSet::with_defaults(),
            content_rules: ContentRuleSet::permissive_defaults(),
        }
    }

    /// Creates a rule engine with only time rules (no content filtering).
    pub fn time_only() -> Self {
        Self {
            time_rules: TimeRuleSet::with_defaults(),
            content_rules: ContentRuleSet::new(),
        }
    }

    /// Creates a rule engine with only content rules (no time restrictions).
    pub fn content_only() -> Self {
        Self {
            time_rules: TimeRuleSet::new(),
            content_rules: ContentRuleSet::family_safe_defaults(),
        }
    }

    /// Evaluates the classification result at the given time.
    ///
    /// Order of evaluation:
    /// 1. Time rules checked first - if blocked, return immediately
    /// 2. Content rules checked against classification matches
    /// 3. Default allow if no rules match
    pub fn evaluate(
        &self,
        classification: &ClassificationResult,
        day: Weekday,
        time: TimeOfDay,
    ) -> RuleEngineResult {
        // Step 1: Check time rules first
        let blocking_time_rules = self.time_rules.blocking_rules(day, time);
        if let Some(first_blocking) = blocking_time_rules.first() {
            return RuleEngineResult::blocked_by_time(first_blocking);
        }

        // Step 2: Check content rules against classification
        if !classification.matches.is_empty() {
            let category_matches: Vec<_> = classification
                .matches
                .iter()
                .map(|m| (m.category, m.confidence))
                .collect();

            let results = self.content_rules.evaluate_all(&category_matches);
            if let Some(first_result) = results.into_iter().next() {
                return RuleEngineResult::from_content_result(first_result);
            }
        }

        // Step 3: Default allow
        RuleEngineResult::allow()
    }

    /// Evaluates at the current time.
    pub fn evaluate_now(&self, classification: &ClassificationResult) -> RuleEngineResult {
        let now = chrono::Local::now();
        let day = Weekday::from_chrono(now.weekday());
        let time = TimeOfDay::new(now.hour() as u8, now.minute() as u8);
        self.evaluate(classification, day, time)
    }

    /// Checks if the current time is blocked by time rules (ignoring content).
    pub fn is_time_blocked(&self, day: Weekday, time: TimeOfDay) -> bool {
        self.time_rules.is_blocked(day, time)
    }

    /// Checks if the current time is blocked by time rules.
    pub fn is_time_blocked_now(&self) -> bool {
        self.time_rules.is_blocked_now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classifier::{Category, CategoryMatch};

    // ==================== RuleAction Tests ====================

    #[test]
    fn rule_action_names() {
        assert_eq!(RuleAction::Allow.name(), "Allow");
        assert_eq!(RuleAction::Warn.name(), "Warn");
        assert_eq!(RuleAction::Block.name(), "Block");
    }

    #[test]
    fn rule_action_default_is_allow() {
        assert_eq!(RuleAction::default(), RuleAction::Allow);
    }

    // ==================== RuleSource Tests ====================

    #[test]
    fn rule_source_none() {
        let source = RuleSource::None;
        assert!(!source.has_rule());
        assert!(!source.is_time_rule());
        assert!(!source.is_content_rule());
        assert!(source.rule_id().is_none());
        assert!(source.rule_name().is_none());
    }

    #[test]
    fn rule_source_time_rule() {
        let source = RuleSource::TimeRule {
            rule_id: "bedtime".to_string(),
            rule_name: "Bedtime".to_string(),
        };
        assert!(source.has_rule());
        assert!(source.is_time_rule());
        assert!(!source.is_content_rule());
        assert_eq!(source.rule_id(), Some("bedtime"));
        assert_eq!(source.rule_name(), Some("Bedtime"));
    }

    #[test]
    fn rule_source_content_rule() {
        let content_result = ContentRuleResult {
            rule_id: "violence_block".to_string(),
            rule_name: "Block Violence".to_string(),
            category: Category::Violence,
            confidence: 0.9,
            action: ContentAction::Block,
        };
        let source = RuleSource::ContentRule(content_result);
        assert!(source.has_rule());
        assert!(!source.is_time_rule());
        assert!(source.is_content_rule());
        assert_eq!(source.rule_id(), Some("violence_block"));
        assert_eq!(source.rule_name(), Some("Block Violence"));
    }

    // ==================== RuleEngineResult Tests ====================

    #[test]
    fn rule_engine_result_allow() {
        let result = RuleEngineResult::allow();
        assert!(result.should_allow());
        assert!(!result.should_block());
        assert!(!result.should_warn());
        assert!(!result.source.has_rule());
    }

    #[test]
    fn rule_engine_result_blocked_by_time() {
        use crate::time_rules::TimeRange;

        let rule = TimeRule::new(
            "bedtime",
            "Bedtime",
            vec![Weekday::Monday],
            TimeRange::from_hours(21, 7),
        );
        let result = RuleEngineResult::blocked_by_time(&rule);
        assert!(result.should_block());
        assert!(!result.should_allow());
        assert!(result.source.is_time_rule());
        assert_eq!(result.source.rule_id(), Some("bedtime"));
    }

    #[test]
    fn rule_engine_result_from_content_result() {
        let content_result = ContentRuleResult {
            rule_id: "violence_block".to_string(),
            rule_name: "Block Violence".to_string(),
            category: Category::Violence,
            confidence: 0.9,
            action: ContentAction::Block,
        };
        let result = RuleEngineResult::from_content_result(content_result);
        assert!(result.should_block());
        assert!(result.source.is_content_rule());
    }

    #[test]
    fn rule_engine_result_warn_from_content() {
        let content_result = ContentRuleResult {
            rule_id: "violence_warn".to_string(),
            rule_name: "Warn Violence".to_string(),
            category: Category::Violence,
            confidence: 0.9,
            action: ContentAction::Warn,
        };
        let result = RuleEngineResult::from_content_result(content_result);
        assert!(result.should_warn());
        assert!(!result.should_block());
    }

    // ==================== RuleEngine Tests ====================

    #[test]
    fn rule_engine_new_is_empty() {
        let engine = RuleEngine::new();
        assert!(engine.time_rules.rules.is_empty());
        assert!(engine.content_rules.rules.is_empty());
    }

    #[test]
    fn rule_engine_with_defaults_has_rules() {
        let engine = RuleEngine::with_defaults();
        assert!(!engine.time_rules.rules.is_empty());
        assert!(!engine.content_rules.rules.is_empty());
    }

    #[test]
    fn rule_engine_time_only_has_time_rules() {
        let engine = RuleEngine::time_only();
        assert!(!engine.time_rules.rules.is_empty());
        assert!(engine.content_rules.rules.is_empty());
    }

    #[test]
    fn rule_engine_content_only_has_content_rules() {
        let engine = RuleEngine::content_only();
        assert!(engine.time_rules.rules.is_empty());
        assert!(!engine.content_rules.rules.is_empty());
    }

    #[test]
    fn rule_engine_empty_allows_everything() {
        let engine = RuleEngine::new();

        // Create a classification with a match
        let matches = vec![CategoryMatch::new(
            Category::Violence,
            0.9,
            Some("kill".to_string()),
        )];
        let classification = ClassificationResult::with_matches(matches, 100);

        // Empty engine should allow
        let result = engine.evaluate(&classification, Weekday::Monday, TimeOfDay::new(12, 0));
        assert!(result.should_allow());
        assert!(!result.source.has_rule());
    }

    #[test]
    fn rule_engine_time_rules_checked_first() {
        let engine = RuleEngine::with_defaults();

        // Empty classification, but during bedtime
        let classification = ClassificationResult::safe(100);

        // Sunday 10pm - should be blocked by bedtime (school night)
        let result = engine.evaluate(&classification, Weekday::Sunday, TimeOfDay::new(22, 0));
        assert!(result.should_block());
        assert!(result.source.is_time_rule());
    }

    #[test]
    fn rule_engine_content_rules_checked_after_time() {
        let engine = RuleEngine::with_defaults();

        // Classification with violence match
        let matches = vec![CategoryMatch::new(
            Category::Violence,
            0.9,
            Some("kill".to_string()),
        )];
        let classification = ClassificationResult::with_matches(matches, 100);

        // Wednesday 3pm - not blocked by time, should be blocked by content
        let result = engine.evaluate(&classification, Weekday::Wednesday, TimeOfDay::new(15, 0));
        assert!(result.should_block());
        assert!(result.source.is_content_rule());
        assert_eq!(result.source.rule_id(), Some("violence_block"));
    }

    #[test]
    fn rule_engine_default_allow_on_no_match() {
        let engine = RuleEngine::with_defaults();

        // Empty classification during allowed time
        let classification = ClassificationResult::safe(100);

        // Wednesday 3pm - no time block, no content match
        let result = engine.evaluate(&classification, Weekday::Wednesday, TimeOfDay::new(15, 0));
        assert!(result.should_allow());
        assert!(!result.source.has_rule());
    }

    #[test]
    fn rule_engine_below_threshold_allows() {
        let engine = RuleEngine::with_defaults();

        // Violence match but below threshold (0.7 is the default)
        let matches = vec![CategoryMatch::new(Category::Violence, 0.5, None)];
        let classification = ClassificationResult::with_matches(matches, 100);

        // Wednesday 3pm - below threshold, should allow
        let result = engine.evaluate(&classification, Weekday::Wednesday, TimeOfDay::new(15, 0));
        assert!(result.should_allow());
    }

    #[test]
    fn rule_engine_permissive_warns_instead_of_blocking() {
        let engine = RuleEngine::with_permissive_content();

        // Violence match above threshold
        let matches = vec![CategoryMatch::new(Category::Violence, 0.9, None)];
        let classification = ClassificationResult::with_matches(matches, 100);

        // Wednesday 3pm - permissive mode warns for violence
        let result = engine.evaluate(&classification, Weekday::Wednesday, TimeOfDay::new(15, 0));
        assert!(result.should_warn());
        assert!(!result.should_block());
    }

    #[test]
    fn rule_engine_selfharm_always_blocks() {
        // Even in permissive mode, self-harm should block
        let engine = RuleEngine::with_permissive_content();

        let matches = vec![CategoryMatch::new(Category::SelfHarm, 0.6, None)];
        let classification = ClassificationResult::with_matches(matches, 100);

        // Wednesday 3pm
        let result = engine.evaluate(&classification, Weekday::Wednesday, TimeOfDay::new(15, 0));
        assert!(result.should_block());
    }

    #[test]
    fn rule_engine_multiple_categories_most_restrictive() {
        let engine = RuleEngine::with_permissive_content();

        // Multiple categories - self-harm (block) and violence (warn)
        let matches = vec![
            CategoryMatch::new(Category::Violence, 0.9, None),
            CategoryMatch::new(Category::SelfHarm, 0.6, None),
        ];
        let classification = ClassificationResult::with_matches(matches, 100);

        // Wednesday 3pm - should block (self-harm is more restrictive)
        let result = engine.evaluate(&classification, Weekday::Wednesday, TimeOfDay::new(15, 0));
        assert!(result.should_block());
    }

    #[test]
    fn rule_engine_time_block_takes_precedence() {
        let engine = RuleEngine::with_defaults();

        // Safe classification
        let classification = ClassificationResult::safe(100);

        // Sunday 10pm (blocked by bedtime)
        let result = engine.evaluate(&classification, Weekday::Sunday, TimeOfDay::new(22, 0));
        assert!(result.should_block());
        assert!(result.source.is_time_rule());

        // But even with harmful content, time rule is reported first
        let matches = vec![CategoryMatch::new(Category::Violence, 0.9, None)];
        let classification = ClassificationResult::with_matches(matches, 100);
        let result = engine.evaluate(&classification, Weekday::Sunday, TimeOfDay::new(22, 0));
        assert!(result.should_block());
        assert!(result.source.is_time_rule()); // Time rule reported, not content
    }

    #[test]
    fn rule_engine_is_time_blocked() {
        let engine = RuleEngine::with_defaults();

        // Sunday 10pm - bedtime
        assert!(engine.is_time_blocked(Weekday::Sunday, TimeOfDay::new(22, 0)));

        // Wednesday 3pm - not bedtime
        assert!(!engine.is_time_blocked(Weekday::Wednesday, TimeOfDay::new(15, 0)));
    }

    // ==================== Serialization Tests ====================

    #[test]
    fn rule_action_serialization() {
        assert_eq!(
            serde_json::to_string(&RuleAction::Block).unwrap(),
            "\"block\""
        );
        assert_eq!(
            serde_json::to_string(&RuleAction::Warn).unwrap(),
            "\"warn\""
        );
        assert_eq!(
            serde_json::to_string(&RuleAction::Allow).unwrap(),
            "\"allow\""
        );
    }

    #[test]
    fn rule_source_serialization() {
        let source = RuleSource::None;
        let json = serde_json::to_string(&source).unwrap();
        let deserialized: RuleSource = serde_json::from_str(&json).unwrap();
        assert_eq!(source, deserialized);

        let source = RuleSource::TimeRule {
            rule_id: "test".to_string(),
            rule_name: "Test".to_string(),
        };
        let json = serde_json::to_string(&source).unwrap();
        let deserialized: RuleSource = serde_json::from_str(&json).unwrap();
        assert_eq!(source, deserialized);
    }

    #[test]
    fn rule_engine_result_serialization() {
        let result = RuleEngineResult::allow();
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: RuleEngineResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.action, deserialized.action);
    }

    #[test]
    fn rule_engine_serialization() {
        let engine = RuleEngine::with_defaults();
        let json = serde_json::to_string(&engine).unwrap();
        let deserialized: RuleEngine = serde_json::from_str(&json).unwrap();
        assert_eq!(
            engine.time_rules.rules.len(),
            deserialized.time_rules.rules.len()
        );
        assert_eq!(
            engine.content_rules.rules.len(),
            deserialized.content_rules.rules.len()
        );
    }
}

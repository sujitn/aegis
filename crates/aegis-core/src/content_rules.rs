//! Content rules for mapping safety categories to actions.
//!
//! This module provides content-based filtering rules that map detected
//! safety categories to specific actions (block, warn, allow) based on
//! configurable confidence thresholds.

use serde::{Deserialize, Serialize};

use crate::classifier::Category;

/// Action to take when a content rule matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentAction {
    /// Block the content entirely.
    #[default]
    Block,
    /// Show a warning but allow the content.
    Warn,
    /// Allow the content without intervention.
    Allow,
}

impl ContentAction {
    /// Returns a human-readable name for this action.
    pub fn name(&self) -> &'static str {
        match self {
            ContentAction::Block => "Block",
            ContentAction::Warn => "Warn",
            ContentAction::Allow => "Allow",
        }
    }
}

/// A single content rule that maps a category to an action with a threshold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentRule {
    /// Unique identifier for this rule.
    pub id: String,
    /// Human-readable name for this rule.
    pub name: String,
    /// The category this rule applies to.
    pub category: Category,
    /// The action to take when the threshold is met.
    pub action: ContentAction,
    /// Minimum confidence threshold (0.0 to 1.0) for the rule to trigger.
    pub threshold: f32,
    /// Whether this rule is currently enabled.
    pub enabled: bool,
}

impl ContentRule {
    /// Creates a new content rule.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        category: Category,
        action: ContentAction,
        threshold: f32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category,
            action,
            threshold: threshold.clamp(0.0, 1.0),
            enabled: true,
        }
    }

    /// Creates a blocking rule for the given category.
    pub fn block(id: impl Into<String>, category: Category, threshold: f32) -> Self {
        Self::new(
            id,
            format!("Block {}", category.name()),
            category,
            ContentAction::Block,
            threshold,
        )
    }

    /// Creates a warning rule for the given category.
    pub fn warn(id: impl Into<String>, category: Category, threshold: f32) -> Self {
        Self::new(
            id,
            format!("Warn {}", category.name()),
            category,
            ContentAction::Warn,
            threshold,
        )
    }

    /// Creates an allow rule for the given category.
    pub fn allow(id: impl Into<String>, category: Category) -> Self {
        Self::new(
            id,
            format!("Allow {}", category.name()),
            category,
            ContentAction::Allow,
            0.0, // Threshold doesn't matter for allow
        )
    }

    /// Sets whether this rule is enabled.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Checks if this rule matches the given category and confidence.
    ///
    /// Returns `Some(action)` if the rule matches, `None` otherwise.
    pub fn matches(&self, category: Category, confidence: f32) -> Option<ContentAction> {
        if !self.enabled {
            return None;
        }

        if self.category == category && confidence >= self.threshold {
            Some(self.action)
        } else {
            None
        }
    }
}

/// A collection of content rules for evaluating classifications.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentRuleSet {
    /// The rules in this set.
    pub rules: Vec<ContentRule>,
}

/// Result of evaluating content against rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentRuleResult {
    /// The rule that matched.
    pub rule_id: String,
    /// The rule name.
    pub rule_name: String,
    /// The category that triggered the rule.
    pub category: Category,
    /// The confidence score that triggered the rule.
    pub confidence: f32,
    /// The action to take.
    pub action: ContentAction,
}

impl ContentRuleSet {
    /// Creates an empty rule set.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Creates a family-safe default rule set.
    ///
    /// Default thresholds:
    /// - Violence: 0.7 (block)
    /// - SelfHarm: 0.5 (block)
    /// - Adult: 0.7 (block)
    /// - Jailbreak: 0.8 (block)
    /// - Hate: 0.7 (block)
    /// - Illegal: 0.7 (block)
    pub fn family_safe_defaults() -> Self {
        Self {
            rules: vec![
                ContentRule::block("violence_block", Category::Violence, 0.7),
                ContentRule::block("selfharm_block", Category::SelfHarm, 0.5),
                ContentRule::block("adult_block", Category::Adult, 0.7),
                ContentRule::block("jailbreak_block", Category::Jailbreak, 0.8),
                ContentRule::block("hate_block", Category::Hate, 0.7),
                ContentRule::block("illegal_block", Category::Illegal, 0.7),
            ],
        }
    }

    /// Creates a permissive rule set that warns but doesn't block.
    pub fn permissive_defaults() -> Self {
        Self {
            rules: vec![
                ContentRule::warn("violence_warn", Category::Violence, 0.8),
                ContentRule::block("selfharm_block", Category::SelfHarm, 0.5), // Always block self-harm
                ContentRule::warn("adult_warn", Category::Adult, 0.8),
                ContentRule::warn("jailbreak_warn", Category::Jailbreak, 0.9),
                ContentRule::warn("hate_warn", Category::Hate, 0.8),
                ContentRule::warn("illegal_warn", Category::Illegal, 0.8),
            ],
        }
    }

    /// Adds a rule to the set.
    pub fn add_rule(&mut self, rule: ContentRule) {
        self.rules.push(rule);
    }

    /// Removes a rule by ID.
    pub fn remove_rule(&mut self, id: &str) -> Option<ContentRule> {
        if let Some(pos) = self.rules.iter().position(|r| r.id == id) {
            Some(self.rules.remove(pos))
        } else {
            None
        }
    }

    /// Gets a rule by ID.
    pub fn get_rule(&self, id: &str) -> Option<&ContentRule> {
        self.rules.iter().find(|r| r.id == id)
    }

    /// Gets a mutable reference to a rule by ID.
    pub fn get_rule_mut(&mut self, id: &str) -> Option<&mut ContentRule> {
        self.rules.iter_mut().find(|r| r.id == id)
    }

    /// Enables or disables a rule by ID.
    pub fn set_rule_enabled(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(rule) = self.get_rule_mut(id) {
            rule.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Updates the threshold for a rule by ID.
    pub fn set_rule_threshold(&mut self, id: &str, threshold: f32) -> bool {
        if let Some(rule) = self.get_rule_mut(id) {
            rule.threshold = threshold.clamp(0.0, 1.0);
            true
        } else {
            false
        }
    }

    /// Updates the action for a rule by ID.
    pub fn set_rule_action(&mut self, id: &str, action: ContentAction) -> bool {
        if let Some(rule) = self.get_rule_mut(id) {
            rule.action = action;
            true
        } else {
            false
        }
    }

    /// Returns all rules for a specific category.
    pub fn rules_for_category(&self, category: Category) -> Vec<&ContentRule> {
        self.rules
            .iter()
            .filter(|r| r.category == category)
            .collect()
    }

    /// Returns all enabled rules.
    pub fn enabled_rules(&self) -> Vec<&ContentRule> {
        self.rules.iter().filter(|r| r.enabled).collect()
    }

    /// Evaluates a single category match against all rules.
    ///
    /// Returns the first matching rule result (most restrictive action wins).
    pub fn evaluate(&self, category: Category, confidence: f32) -> Option<ContentRuleResult> {
        // Find all matching rules
        let mut results: Vec<ContentRuleResult> = self
            .rules
            .iter()
            .filter_map(|rule| {
                rule.matches(category, confidence)
                    .map(|action| ContentRuleResult {
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        category,
                        confidence,
                        action,
                    })
            })
            .collect();

        // Sort by action priority: Block > Warn > Allow
        results.sort_by_key(|r| match r.action {
            ContentAction::Block => 0,
            ContentAction::Warn => 1,
            ContentAction::Allow => 2,
        });

        results.into_iter().next()
    }

    /// Evaluates multiple category matches and returns all rule results.
    ///
    /// Results are sorted by action priority (Block first).
    pub fn evaluate_all(&self, matches: &[(Category, f32)]) -> Vec<ContentRuleResult> {
        let mut results: Vec<ContentRuleResult> = matches
            .iter()
            .filter_map(|(category, confidence)| self.evaluate(*category, *confidence))
            .collect();

        // Sort by action priority
        results.sort_by_key(|r| match r.action {
            ContentAction::Block => 0,
            ContentAction::Warn => 1,
            ContentAction::Allow => 2,
        });

        results
    }

    /// Returns the most restrictive action from evaluating all matches.
    pub fn most_restrictive_action(&self, matches: &[(Category, f32)]) -> Option<ContentAction> {
        self.evaluate_all(matches).first().map(|r| r.action)
    }

    /// Returns true if any match would result in a block.
    pub fn should_block(&self, matches: &[(Category, f32)]) -> bool {
        self.most_restrictive_action(matches) == Some(ContentAction::Block)
    }

    /// Returns true if any match would result in a warning (but not a block).
    pub fn should_warn(&self, matches: &[(Category, f32)]) -> bool {
        self.most_restrictive_action(matches) == Some(ContentAction::Warn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_action_names() {
        assert_eq!(ContentAction::Block.name(), "Block");
        assert_eq!(ContentAction::Warn.name(), "Warn");
        assert_eq!(ContentAction::Allow.name(), "Allow");
    }

    #[test]
    fn content_action_default_is_block() {
        assert_eq!(ContentAction::default(), ContentAction::Block);
    }

    #[test]
    fn content_rule_new_clamps_threshold() {
        let rule = ContentRule::new(
            "test",
            "Test",
            Category::Violence,
            ContentAction::Block,
            1.5,
        );
        assert_eq!(rule.threshold, 1.0);

        let rule = ContentRule::new(
            "test",
            "Test",
            Category::Violence,
            ContentAction::Block,
            -0.5,
        );
        assert_eq!(rule.threshold, 0.0);
    }

    #[test]
    fn content_rule_block_helper() {
        let rule = ContentRule::block("vio", Category::Violence, 0.7);
        assert_eq!(rule.id, "vio");
        assert_eq!(rule.category, Category::Violence);
        assert_eq!(rule.action, ContentAction::Block);
        assert_eq!(rule.threshold, 0.7);
        assert!(rule.enabled);
    }

    #[test]
    fn content_rule_warn_helper() {
        let rule = ContentRule::warn("adult", Category::Adult, 0.8);
        assert_eq!(rule.action, ContentAction::Warn);
    }

    #[test]
    fn content_rule_allow_helper() {
        let rule = ContentRule::allow("hate", Category::Hate);
        assert_eq!(rule.action, ContentAction::Allow);
        assert_eq!(rule.threshold, 0.0);
    }

    #[test]
    fn content_rule_matches_when_enabled_and_above_threshold() {
        let rule = ContentRule::block("test", Category::Violence, 0.7);

        // Above threshold - matches
        assert_eq!(
            rule.matches(Category::Violence, 0.8),
            Some(ContentAction::Block)
        );

        // At threshold - matches
        assert_eq!(
            rule.matches(Category::Violence, 0.7),
            Some(ContentAction::Block)
        );

        // Below threshold - no match
        assert_eq!(rule.matches(Category::Violence, 0.6), None);

        // Wrong category - no match
        assert_eq!(rule.matches(Category::Adult, 0.9), None);
    }

    #[test]
    fn content_rule_disabled_does_not_match() {
        let rule = ContentRule::block("test", Category::Violence, 0.7).with_enabled(false);
        assert_eq!(rule.matches(Category::Violence, 0.9), None);
    }

    #[test]
    fn content_rule_set_family_safe_defaults() {
        let rules = ContentRuleSet::family_safe_defaults();
        assert_eq!(rules.rules.len(), 6);

        // Check specific thresholds
        let violence = rules.get_rule("violence_block").unwrap();
        assert_eq!(violence.threshold, 0.7);

        let selfharm = rules.get_rule("selfharm_block").unwrap();
        assert_eq!(selfharm.threshold, 0.5); // Lower threshold for self-harm

        let jailbreak = rules.get_rule("jailbreak_block").unwrap();
        assert_eq!(jailbreak.threshold, 0.8);
    }

    #[test]
    fn content_rule_set_permissive_defaults() {
        let rules = ContentRuleSet::permissive_defaults();
        assert_eq!(rules.rules.len(), 6);

        // Violence should warn, not block
        let violence = rules.get_rule("violence_warn").unwrap();
        assert_eq!(violence.action, ContentAction::Warn);

        // Self-harm should still block
        let selfharm = rules.get_rule("selfharm_block").unwrap();
        assert_eq!(selfharm.action, ContentAction::Block);
    }

    #[test]
    fn content_rule_set_add_remove_rules() {
        let mut rules = ContentRuleSet::new();
        assert_eq!(rules.rules.len(), 0);

        rules.add_rule(ContentRule::block("test", Category::Violence, 0.7));
        assert_eq!(rules.rules.len(), 1);

        let removed = rules.remove_rule("test");
        assert!(removed.is_some());
        assert_eq!(rules.rules.len(), 0);

        let removed = rules.remove_rule("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn content_rule_set_enable_disable_rules() {
        let mut rules = ContentRuleSet::family_safe_defaults();

        assert!(rules.get_rule("violence_block").unwrap().enabled);

        rules.set_rule_enabled("violence_block", false);
        assert!(!rules.get_rule("violence_block").unwrap().enabled);

        rules.set_rule_enabled("violence_block", true);
        assert!(rules.get_rule("violence_block").unwrap().enabled);

        // Non-existent rule
        assert!(!rules.set_rule_enabled("nonexistent", false));
    }

    #[test]
    fn content_rule_set_update_threshold() {
        let mut rules = ContentRuleSet::family_safe_defaults();

        assert!(rules.set_rule_threshold("violence_block", 0.9));
        assert_eq!(rules.get_rule("violence_block").unwrap().threshold, 0.9);

        // Clamping
        assert!(rules.set_rule_threshold("violence_block", 1.5));
        assert_eq!(rules.get_rule("violence_block").unwrap().threshold, 1.0);

        // Non-existent rule
        assert!(!rules.set_rule_threshold("nonexistent", 0.5));
    }

    #[test]
    fn content_rule_set_update_action() {
        let mut rules = ContentRuleSet::family_safe_defaults();

        assert!(rules.set_rule_action("violence_block", ContentAction::Warn));
        assert_eq!(
            rules.get_rule("violence_block").unwrap().action,
            ContentAction::Warn
        );

        // Non-existent rule
        assert!(!rules.set_rule_action("nonexistent", ContentAction::Allow));
    }

    #[test]
    fn content_rule_set_rules_for_category() {
        let mut rules = ContentRuleSet::new();
        rules.add_rule(ContentRule::block("vio1", Category::Violence, 0.7));
        rules.add_rule(ContentRule::warn("vio2", Category::Violence, 0.5));
        rules.add_rule(ContentRule::block("adult", Category::Adult, 0.7));

        let violence_rules = rules.rules_for_category(Category::Violence);
        assert_eq!(violence_rules.len(), 2);

        let adult_rules = rules.rules_for_category(Category::Adult);
        assert_eq!(adult_rules.len(), 1);
    }

    #[test]
    fn content_rule_set_enabled_rules() {
        let mut rules = ContentRuleSet::family_safe_defaults();
        assert_eq!(rules.enabled_rules().len(), 6);

        rules.set_rule_enabled("violence_block", false);
        rules.set_rule_enabled("adult_block", false);
        assert_eq!(rules.enabled_rules().len(), 4);
    }

    #[test]
    fn content_rule_set_evaluate_single() {
        let rules = ContentRuleSet::family_safe_defaults();

        // Above threshold - should match
        let result = rules.evaluate(Category::Violence, 0.8);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.category, Category::Violence);
        assert_eq!(result.action, ContentAction::Block);

        // Below threshold - no match
        let result = rules.evaluate(Category::Violence, 0.5);
        assert!(result.is_none());
    }

    #[test]
    fn content_rule_set_evaluate_prioritizes_block() {
        let mut rules = ContentRuleSet::new();
        // Add both warn and block rules for same category
        rules.add_rule(ContentRule::warn("warn", Category::Violence, 0.5));
        rules.add_rule(ContentRule::block("block", Category::Violence, 0.7));

        // Above block threshold - should get block
        let result = rules.evaluate(Category::Violence, 0.8);
        assert_eq!(result.unwrap().action, ContentAction::Block);

        // Above warn but below block - should get warn
        let result = rules.evaluate(Category::Violence, 0.6);
        assert_eq!(result.unwrap().action, ContentAction::Warn);
    }

    #[test]
    fn content_rule_set_evaluate_all() {
        let rules = ContentRuleSet::family_safe_defaults();

        let matches = vec![
            (Category::Violence, 0.8),
            (Category::Adult, 0.9),
            (Category::Hate, 0.5), // Below threshold
        ];

        let results = rules.evaluate_all(&matches);
        assert_eq!(results.len(), 2); // Violence and Adult
        assert!(results.iter().all(|r| r.action == ContentAction::Block));
    }

    #[test]
    fn content_rule_set_most_restrictive_action() {
        let mut rules = ContentRuleSet::new();
        rules.add_rule(ContentRule::warn("warn", Category::Adult, 0.5));
        rules.add_rule(ContentRule::block("block", Category::Violence, 0.7));

        // Both match - block is more restrictive
        let matches = vec![(Category::Violence, 0.8), (Category::Adult, 0.6)];
        assert_eq!(
            rules.most_restrictive_action(&matches),
            Some(ContentAction::Block)
        );

        // Only warn matches
        let matches = vec![(Category::Adult, 0.6)];
        assert_eq!(
            rules.most_restrictive_action(&matches),
            Some(ContentAction::Warn)
        );

        // Nothing matches
        let matches = vec![(Category::Hate, 0.9)];
        assert_eq!(rules.most_restrictive_action(&matches), None);
    }

    #[test]
    fn content_rule_set_should_block() {
        let rules = ContentRuleSet::family_safe_defaults();

        let matches = vec![(Category::Violence, 0.8)];
        assert!(rules.should_block(&matches));

        let matches = vec![(Category::Violence, 0.5)]; // Below threshold
        assert!(!rules.should_block(&matches));
    }

    #[test]
    fn content_rule_set_should_warn() {
        let rules = ContentRuleSet::permissive_defaults();

        // Violence is set to warn in permissive
        let matches = vec![(Category::Violence, 0.9)];
        assert!(rules.should_warn(&matches));

        // Self-harm is set to block even in permissive
        let matches = vec![(Category::SelfHarm, 0.6)];
        assert!(!rules.should_warn(&matches)); // Block, not warn
        assert!(rules.should_block(&matches));
    }

    #[test]
    fn content_rule_serialization() {
        let rule = ContentRule::block("test", Category::Violence, 0.7);
        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: ContentRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, deserialized);
    }

    #[test]
    fn content_rule_set_serialization() {
        let rules = ContentRuleSet::family_safe_defaults();
        let json = serde_json::to_string(&rules).unwrap();
        let deserialized: ContentRuleSet = serde_json::from_str(&json).unwrap();
        assert_eq!(rules.rules.len(), deserialized.rules.len());
    }

    #[test]
    fn content_action_serialization() {
        assert_eq!(
            serde_json::to_string(&ContentAction::Block).unwrap(),
            "\"block\""
        );
        assert_eq!(
            serde_json::to_string(&ContentAction::Warn).unwrap(),
            "\"warn\""
        );
        assert_eq!(
            serde_json::to_string(&ContentAction::Allow).unwrap(),
            "\"allow\""
        );
    }
}

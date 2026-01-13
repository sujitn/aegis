//! Time-based rules for blocking AI access.
//!
//! This module provides functionality to define and evaluate time-based rules
//! for controlling when AI access is blocked (e.g., bedtime, school hours).

use chrono::{Datelike, NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Days of the week for rule scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    /// Returns all weekdays (Monday through Friday).
    pub fn weekdays() -> Vec<Weekday> {
        vec![
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
        ]
    }

    /// Returns weekend days (Saturday and Sunday).
    pub fn weekends() -> Vec<Weekday> {
        vec![Weekday::Saturday, Weekday::Sunday]
    }

    /// Returns all days of the week.
    pub fn all() -> Vec<Weekday> {
        vec![
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
            Weekday::Saturday,
            Weekday::Sunday,
        ]
    }

    /// Returns school nights (Sunday through Thursday evenings).
    pub fn school_nights() -> Vec<Weekday> {
        vec![
            Weekday::Sunday,
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
        ]
    }

    /// Converts from chrono's Weekday.
    pub fn from_chrono(weekday: chrono::Weekday) -> Self {
        match weekday {
            chrono::Weekday::Mon => Weekday::Monday,
            chrono::Weekday::Tue => Weekday::Tuesday,
            chrono::Weekday::Wed => Weekday::Wednesday,
            chrono::Weekday::Thu => Weekday::Thursday,
            chrono::Weekday::Fri => Weekday::Friday,
            chrono::Weekday::Sat => Weekday::Saturday,
            chrono::Weekday::Sun => Weekday::Sunday,
        }
    }
}

/// Time of day represented as hour and minute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeOfDay {
    /// Hour (0-23).
    pub hour: u8,
    /// Minute (0-59).
    pub minute: u8,
}

impl TimeOfDay {
    /// Creates a new TimeOfDay.
    ///
    /// # Panics
    /// Panics if hour >= 24 or minute >= 60.
    pub fn new(hour: u8, minute: u8) -> Self {
        assert!(hour < 24, "hour must be 0-23");
        assert!(minute < 60, "minute must be 0-59");
        Self { hour, minute }
    }

    /// Creates a TimeOfDay from hour only (minute = 0).
    pub fn from_hour(hour: u8) -> Self {
        Self::new(hour, 0)
    }

    /// Converts to minutes since midnight for comparison.
    pub fn to_minutes(&self) -> u16 {
        self.hour as u16 * 60 + self.minute as u16
    }

    /// Creates from a chrono NaiveTime.
    pub fn from_naive_time(time: NaiveTime) -> Self {
        Self {
            hour: time.hour() as u8,
            minute: time.minute() as u8,
        }
    }
}

impl PartialOrd for TimeOfDay {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimeOfDay {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_minutes().cmp(&other.to_minutes())
    }
}

/// A time range with start and end times.
///
/// Supports overnight ranges where end < start (e.g., 9pm-7am).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange {
    /// Start time of the range.
    pub start: TimeOfDay,
    /// End time of the range.
    pub end: TimeOfDay,
}

impl TimeRange {
    /// Creates a new time range.
    pub fn new(start: TimeOfDay, end: TimeOfDay) -> Self {
        Self { start, end }
    }

    /// Creates a time range from hour values.
    pub fn from_hours(start_hour: u8, end_hour: u8) -> Self {
        Self {
            start: TimeOfDay::from_hour(start_hour),
            end: TimeOfDay::from_hour(end_hour),
        }
    }

    /// Returns true if this is an overnight range (crosses midnight).
    pub fn is_overnight(&self) -> bool {
        self.end < self.start
    }

    /// Checks if a given time falls within this range.
    ///
    /// Handles overnight ranges correctly (e.g., 21:00-07:00).
    pub fn contains(&self, time: TimeOfDay) -> bool {
        if self.is_overnight() {
            // Overnight range: 21:00-07:00 means blocked if >= 21:00 OR < 07:00
            time >= self.start || time < self.end
        } else {
            // Normal range: 08:00-15:00 means blocked if >= 08:00 AND < 15:00
            time >= self.start && time < self.end
        }
    }
}

/// A single time-based rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRule {
    /// Unique identifier for the rule.
    pub id: String,
    /// Human-readable name for the rule.
    pub name: String,
    /// Days this rule applies to.
    pub days: HashSet<Weekday>,
    /// Time range when access is blocked.
    pub time_range: TimeRange,
    /// Whether this rule is currently enabled.
    pub enabled: bool,
}

impl TimeRule {
    /// Creates a new time rule.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        days: Vec<Weekday>,
        time_range: TimeRange,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            days: days.into_iter().collect(),
            time_range,
            enabled: true,
        }
    }

    /// Disables this rule.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Enables this rule.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Checks if access is blocked at the given day and time.
    pub fn is_blocked(&self, day: Weekday, time: TimeOfDay) -> bool {
        if !self.enabled {
            return false;
        }

        // For overnight ranges, we need special handling.
        // If it's an overnight range (e.g., 21:00-07:00):
        // - The "start day" is when you enter the blocked period (at 21:00)
        // - The "end day" is the next day (when you exit at 07:00)
        if self.time_range.is_overnight() {
            // Check if we're in the "start day" portion (after start time)
            if time >= self.time_range.start && self.days.contains(&day) {
                return true;
            }
            // Check if we're in the "end day" portion (before end time)
            // This means the rule was configured for the previous day
            if time < self.time_range.end {
                let prev_day = previous_day(day);
                if self.days.contains(&prev_day) {
                    return true;
                }
            }
            false
        } else {
            // Normal range: just check if day matches and time is in range
            self.days.contains(&day) && self.time_range.contains(time)
        }
    }
}

/// Returns the previous day of the week.
fn previous_day(day: Weekday) -> Weekday {
    match day {
        Weekday::Monday => Weekday::Sunday,
        Weekday::Tuesday => Weekday::Monday,
        Weekday::Wednesday => Weekday::Tuesday,
        Weekday::Thursday => Weekday::Wednesday,
        Weekday::Friday => Weekday::Thursday,
        Weekday::Saturday => Weekday::Friday,
        Weekday::Sunday => Weekday::Saturday,
    }
}

/// A collection of time rules that can be evaluated together.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimeRuleSet {
    /// The rules in this set.
    pub rules: Vec<TimeRule>,
}

impl TimeRuleSet {
    /// Creates an empty rule set.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Creates a rule set with default presets.
    pub fn with_defaults() -> Self {
        let mut set = Self::new();
        set.add_rule(Self::bedtime_school_nights());
        set.add_rule(Self::bedtime_weekends());
        set
    }

    /// Creates the default bedtime rule for school nights (Sun-Thu, 9pm-7am).
    pub fn bedtime_school_nights() -> TimeRule {
        TimeRule::new(
            "bedtime_school",
            "Bedtime (School Nights)",
            Weekday::school_nights(),
            TimeRange::from_hours(21, 7), // 9pm - 7am
        )
    }

    /// Creates the default bedtime rule for weekends (Fri-Sat, 11pm-8am).
    pub fn bedtime_weekends() -> TimeRule {
        TimeRule::new(
            "bedtime_weekend",
            "Bedtime (Weekends)",
            vec![Weekday::Friday, Weekday::Saturday],
            TimeRange::from_hours(23, 8), // 11pm - 8am
        )
    }

    /// Creates a school hours rule (Mon-Fri, 8am-3pm) - disabled by default.
    pub fn school_hours() -> TimeRule {
        let mut rule = TimeRule::new(
            "school_hours",
            "School Hours",
            Weekday::weekdays(),
            TimeRange::from_hours(8, 15), // 8am - 3pm
        );
        rule.disable();
        rule
    }

    /// Adds a rule to the set.
    pub fn add_rule(&mut self, rule: TimeRule) {
        self.rules.push(rule);
    }

    /// Removes a rule by ID.
    pub fn remove_rule(&mut self, id: &str) -> Option<TimeRule> {
        if let Some(pos) = self.rules.iter().position(|r| r.id == id) {
            Some(self.rules.remove(pos))
        } else {
            None
        }
    }

    /// Gets a rule by ID.
    pub fn get_rule(&self, id: &str) -> Option<&TimeRule> {
        self.rules.iter().find(|r| r.id == id)
    }

    /// Gets a mutable reference to a rule by ID.
    pub fn get_rule_mut(&mut self, id: &str) -> Option<&mut TimeRule> {
        self.rules.iter_mut().find(|r| r.id == id)
    }

    /// Checks if access is blocked at the given day and time by any rule.
    pub fn is_blocked(&self, day: Weekday, time: TimeOfDay) -> bool {
        self.rules.iter().any(|rule| rule.is_blocked(day, time))
    }

    /// Checks if access is blocked at the current time.
    pub fn is_blocked_now(&self) -> bool {
        let now = chrono::Local::now();
        let day = Weekday::from_chrono(now.weekday());
        let time = TimeOfDay::new(now.hour() as u8, now.minute() as u8);
        self.is_blocked(day, time)
    }

    /// Returns which rules are blocking at the given day and time.
    pub fn blocking_rules(&self, day: Weekday, time: TimeOfDay) -> Vec<&TimeRule> {
        self.rules
            .iter()
            .filter(|rule| rule.is_blocked(day, time))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== TimeOfDay Tests ====================

    #[test]
    fn time_of_day_creation() {
        let time = TimeOfDay::new(14, 30);
        assert_eq!(time.hour, 14);
        assert_eq!(time.minute, 30);
    }

    #[test]
    fn time_of_day_from_hour() {
        let time = TimeOfDay::from_hour(9);
        assert_eq!(time.hour, 9);
        assert_eq!(time.minute, 0);
    }

    #[test]
    #[should_panic(expected = "hour must be 0-23")]
    fn time_of_day_invalid_hour() {
        TimeOfDay::new(24, 0);
    }

    #[test]
    #[should_panic(expected = "minute must be 0-59")]
    fn time_of_day_invalid_minute() {
        TimeOfDay::new(12, 60);
    }

    #[test]
    fn time_of_day_comparison() {
        let morning = TimeOfDay::new(8, 0);
        let noon = TimeOfDay::new(12, 0);
        let afternoon = TimeOfDay::new(14, 30);

        assert!(morning < noon);
        assert!(noon < afternoon);
        assert!(morning < afternoon);
    }

    #[test]
    fn time_of_day_to_minutes() {
        assert_eq!(TimeOfDay::new(0, 0).to_minutes(), 0);
        assert_eq!(TimeOfDay::new(1, 0).to_minutes(), 60);
        assert_eq!(TimeOfDay::new(12, 30).to_minutes(), 750);
        assert_eq!(TimeOfDay::new(23, 59).to_minutes(), 1439);
    }

    // ==================== Weekday Tests ====================

    #[test]
    fn weekday_groups() {
        assert_eq!(Weekday::weekdays().len(), 5);
        assert_eq!(Weekday::weekends().len(), 2);
        assert_eq!(Weekday::all().len(), 7);
        assert_eq!(Weekday::school_nights().len(), 5);
    }

    #[test]
    fn weekday_school_nights_correct() {
        let school_nights = Weekday::school_nights();
        assert!(school_nights.contains(&Weekday::Sunday));
        assert!(school_nights.contains(&Weekday::Monday));
        assert!(school_nights.contains(&Weekday::Tuesday));
        assert!(school_nights.contains(&Weekday::Wednesday));
        assert!(school_nights.contains(&Weekday::Thursday));
        assert!(!school_nights.contains(&Weekday::Friday));
        assert!(!school_nights.contains(&Weekday::Saturday));
    }

    // ==================== TimeRange Tests ====================

    #[test]
    fn time_range_normal() {
        let range = TimeRange::from_hours(8, 15);
        assert!(!range.is_overnight());
    }

    #[test]
    fn time_range_overnight() {
        let range = TimeRange::from_hours(21, 7);
        assert!(range.is_overnight());
    }

    #[test]
    fn time_range_contains_normal() {
        let range = TimeRange::from_hours(8, 15); // 8am - 3pm

        assert!(!range.contains(TimeOfDay::new(7, 59))); // Before
        assert!(range.contains(TimeOfDay::new(8, 0))); // Start (inclusive)
        assert!(range.contains(TimeOfDay::new(12, 0))); // Middle
        assert!(range.contains(TimeOfDay::new(14, 59))); // Just before end
        assert!(!range.contains(TimeOfDay::new(15, 0))); // End (exclusive)
        assert!(!range.contains(TimeOfDay::new(20, 0))); // After
    }

    #[test]
    fn time_range_contains_overnight() {
        let range = TimeRange::from_hours(21, 7); // 9pm - 7am

        assert!(!range.contains(TimeOfDay::new(20, 59))); // Before start
        assert!(range.contains(TimeOfDay::new(21, 0))); // Start (inclusive)
        assert!(range.contains(TimeOfDay::new(23, 59))); // Late night
        assert!(range.contains(TimeOfDay::new(0, 0))); // Midnight
        assert!(range.contains(TimeOfDay::new(3, 0))); // Early morning
        assert!(range.contains(TimeOfDay::new(6, 59))); // Just before end
        assert!(!range.contains(TimeOfDay::new(7, 0))); // End (exclusive)
        assert!(!range.contains(TimeOfDay::new(12, 0))); // Afternoon
    }

    // ==================== TimeRule Tests ====================

    #[test]
    fn time_rule_creation() {
        let rule = TimeRule::new(
            "test",
            "Test Rule",
            vec![Weekday::Monday, Weekday::Wednesday],
            TimeRange::from_hours(8, 17),
        );

        assert_eq!(rule.id, "test");
        assert_eq!(rule.name, "Test Rule");
        assert!(rule.enabled);
        assert!(rule.days.contains(&Weekday::Monday));
        assert!(rule.days.contains(&Weekday::Wednesday));
        assert!(!rule.days.contains(&Weekday::Tuesday));
    }

    #[test]
    fn time_rule_enable_disable() {
        let mut rule = TimeRule::new(
            "test",
            "Test",
            vec![Weekday::Monday],
            TimeRange::from_hours(8, 17),
        );

        assert!(rule.enabled);
        rule.disable();
        assert!(!rule.enabled);
        rule.enable();
        assert!(rule.enabled);
    }

    #[test]
    fn time_rule_blocks_correct_time() {
        let rule = TimeRule::new(
            "work",
            "Work Hours",
            vec![Weekday::Monday, Weekday::Tuesday, Weekday::Wednesday],
            TimeRange::from_hours(9, 17),
        );

        // Should block on Monday at 10am
        assert!(rule.is_blocked(Weekday::Monday, TimeOfDay::new(10, 0)));
        // Should not block on Monday at 8am (before range)
        assert!(!rule.is_blocked(Weekday::Monday, TimeOfDay::new(8, 0)));
        // Should not block on Monday at 5pm (end of range, exclusive)
        assert!(!rule.is_blocked(Weekday::Monday, TimeOfDay::new(17, 0)));
        // Should not block on Thursday (wrong day)
        assert!(!rule.is_blocked(Weekday::Thursday, TimeOfDay::new(10, 0)));
    }

    #[test]
    fn time_rule_disabled_doesnt_block() {
        let mut rule = TimeRule::new(
            "test",
            "Test",
            vec![Weekday::Monday],
            TimeRange::from_hours(0, 23),
        );
        rule.disable();

        // Disabled rule should never block
        assert!(!rule.is_blocked(Weekday::Monday, TimeOfDay::new(12, 0)));
    }

    #[test]
    fn time_rule_overnight_blocks_correctly() {
        // Bedtime rule: Sunday night 9pm - Monday 7am
        let rule = TimeRule::new(
            "bedtime",
            "Bedtime",
            vec![Weekday::Sunday],
            TimeRange::from_hours(21, 7),
        );

        // Sunday at 10pm - should block (start day, after start time)
        assert!(rule.is_blocked(Weekday::Sunday, TimeOfDay::new(22, 0)));

        // Monday at 3am - should block (day after, before end time)
        assert!(rule.is_blocked(Weekday::Monday, TimeOfDay::new(3, 0)));

        // Monday at 8am - should not block (after end time)
        assert!(!rule.is_blocked(Weekday::Monday, TimeOfDay::new(8, 0)));

        // Sunday at 8pm - should not block (before start time)
        assert!(!rule.is_blocked(Weekday::Sunday, TimeOfDay::new(20, 0)));

        // Saturday at 10pm - should not block (wrong day)
        assert!(!rule.is_blocked(Weekday::Saturday, TimeOfDay::new(22, 0)));
    }

    #[test]
    fn time_rule_overnight_multiple_days() {
        // School night bedtime: Sun-Thu 9pm to next morning 7am
        let rule = TimeRule::new(
            "bedtime",
            "Bedtime",
            Weekday::school_nights(),
            TimeRange::from_hours(21, 7),
        );

        // Sunday 10pm -> Monday 6am blocked
        assert!(rule.is_blocked(Weekday::Sunday, TimeOfDay::new(22, 0)));
        assert!(rule.is_blocked(Weekday::Monday, TimeOfDay::new(6, 0)));

        // Monday 10pm -> Tuesday 6am blocked
        assert!(rule.is_blocked(Weekday::Monday, TimeOfDay::new(22, 0)));
        assert!(rule.is_blocked(Weekday::Tuesday, TimeOfDay::new(6, 0)));

        // Thursday 10pm -> Friday 6am blocked
        assert!(rule.is_blocked(Weekday::Thursday, TimeOfDay::new(22, 0)));
        assert!(rule.is_blocked(Weekday::Friday, TimeOfDay::new(6, 0)));

        // Friday 10pm -> Saturday 6am NOT blocked (Friday not in school_nights)
        assert!(!rule.is_blocked(Weekday::Friday, TimeOfDay::new(22, 0)));
        assert!(!rule.is_blocked(Weekday::Saturday, TimeOfDay::new(6, 0)));
    }

    // ==================== TimeRuleSet Tests ====================

    #[test]
    fn rule_set_empty() {
        let set = TimeRuleSet::new();
        assert!(set.rules.is_empty());
        // Empty set never blocks
        assert!(!set.is_blocked(Weekday::Monday, TimeOfDay::new(12, 0)));
    }

    #[test]
    fn rule_set_add_and_remove() {
        let mut set = TimeRuleSet::new();
        let rule = TimeRule::new(
            "test",
            "Test",
            vec![Weekday::Monday],
            TimeRange::from_hours(8, 17),
        );

        set.add_rule(rule);
        assert_eq!(set.rules.len(), 1);

        let removed = set.remove_rule("test");
        assert!(removed.is_some());
        assert!(set.rules.is_empty());

        // Remove non-existent rule
        let removed = set.remove_rule("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn rule_set_get_rule() {
        let mut set = TimeRuleSet::new();
        set.add_rule(TimeRule::new(
            "test",
            "Test",
            vec![Weekday::Monday],
            TimeRange::from_hours(8, 17),
        ));

        assert!(set.get_rule("test").is_some());
        assert!(set.get_rule("nonexistent").is_none());
    }

    #[test]
    fn rule_set_multiple_rules() {
        let mut set = TimeRuleSet::new();

        // Morning rule
        set.add_rule(TimeRule::new(
            "morning",
            "Morning",
            vec![Weekday::Monday],
            TimeRange::from_hours(8, 12),
        ));

        // Afternoon rule
        set.add_rule(TimeRule::new(
            "afternoon",
            "Afternoon",
            vec![Weekday::Monday],
            TimeRange::from_hours(14, 18),
        ));

        // Monday 10am - blocked by morning
        assert!(set.is_blocked(Weekday::Monday, TimeOfDay::new(10, 0)));
        // Monday 15pm - blocked by afternoon
        assert!(set.is_blocked(Weekday::Monday, TimeOfDay::new(15, 0)));
        // Monday 13pm - not blocked (gap between rules)
        assert!(!set.is_blocked(Weekday::Monday, TimeOfDay::new(13, 0)));
        // Tuesday 10am - not blocked (different day)
        assert!(!set.is_blocked(Weekday::Tuesday, TimeOfDay::new(10, 0)));
    }

    #[test]
    fn rule_set_blocking_rules() {
        let mut set = TimeRuleSet::new();

        set.add_rule(TimeRule::new(
            "rule1",
            "Rule 1",
            vec![Weekday::Monday],
            TimeRange::from_hours(8, 17),
        ));

        set.add_rule(TimeRule::new(
            "rule2",
            "Rule 2",
            vec![Weekday::Monday],
            TimeRange::from_hours(10, 14),
        ));

        // Both rules block at 11am on Monday
        let blocking = set.blocking_rules(Weekday::Monday, TimeOfDay::new(11, 0));
        assert_eq!(blocking.len(), 2);

        // Only rule1 blocks at 9am on Monday
        let blocking = set.blocking_rules(Weekday::Monday, TimeOfDay::new(9, 0));
        assert_eq!(blocking.len(), 1);
        assert_eq!(blocking[0].id, "rule1");
    }

    // ==================== Default Preset Tests ====================

    #[test]
    fn preset_bedtime_school_nights() {
        let rule = TimeRuleSet::bedtime_school_nights();
        assert_eq!(rule.id, "bedtime_school");
        assert!(rule.enabled);

        // Check days
        assert!(rule.days.contains(&Weekday::Sunday));
        assert!(rule.days.contains(&Weekday::Monday));
        assert!(rule.days.contains(&Weekday::Thursday));
        assert!(!rule.days.contains(&Weekday::Friday));
        assert!(!rule.days.contains(&Weekday::Saturday));

        // Check time range (9pm - 7am)
        assert_eq!(rule.time_range.start.hour, 21);
        assert_eq!(rule.time_range.end.hour, 7);
    }

    #[test]
    fn preset_bedtime_weekends() {
        let rule = TimeRuleSet::bedtime_weekends();
        assert_eq!(rule.id, "bedtime_weekend");
        assert!(rule.enabled);

        // Check days
        assert!(rule.days.contains(&Weekday::Friday));
        assert!(rule.days.contains(&Weekday::Saturday));
        assert!(!rule.days.contains(&Weekday::Sunday));

        // Check time range (11pm - 8am)
        assert_eq!(rule.time_range.start.hour, 23);
        assert_eq!(rule.time_range.end.hour, 8);
    }

    #[test]
    fn preset_school_hours_disabled() {
        let rule = TimeRuleSet::school_hours();
        assert_eq!(rule.id, "school_hours");
        assert!(!rule.enabled); // Disabled by default

        // Check time range (8am - 3pm)
        assert_eq!(rule.time_range.start.hour, 8);
        assert_eq!(rule.time_range.end.hour, 15);
    }

    #[test]
    fn rule_set_with_defaults() {
        let set = TimeRuleSet::with_defaults();

        // Should have two rules: bedtime school nights and bedtime weekends
        assert_eq!(set.rules.len(), 2);
        assert!(set.get_rule("bedtime_school").is_some());
        assert!(set.get_rule("bedtime_weekend").is_some());
    }

    #[test]
    fn defaults_block_correctly() {
        let set = TimeRuleSet::with_defaults();

        // Sunday 10pm - blocked (school night bedtime)
        assert!(set.is_blocked(Weekday::Sunday, TimeOfDay::new(22, 0)));

        // Monday 6am - blocked (school night bedtime, morning after)
        assert!(set.is_blocked(Weekday::Monday, TimeOfDay::new(6, 0)));

        // Friday 11:30pm - blocked (weekend bedtime)
        assert!(set.is_blocked(Weekday::Friday, TimeOfDay::new(23, 30)));

        // Saturday 7am - blocked (weekend bedtime morning after)
        assert!(set.is_blocked(Weekday::Saturday, TimeOfDay::new(7, 0)));

        // Wednesday 3pm - not blocked (daytime)
        assert!(!set.is_blocked(Weekday::Wednesday, TimeOfDay::new(15, 0)));

        // Saturday 2pm - not blocked (weekend daytime)
        assert!(!set.is_blocked(Weekday::Saturday, TimeOfDay::new(14, 0)));
    }

    // ==================== Serialization Tests ====================

    #[test]
    fn weekday_serialization() {
        let day = Weekday::Monday;
        let json = serde_json::to_string(&day).unwrap();
        assert_eq!(json, "\"monday\"");

        let deserialized: Weekday = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Weekday::Monday);
    }

    #[test]
    fn time_rule_serialization() {
        let rule = TimeRule::new(
            "test",
            "Test Rule",
            vec![Weekday::Monday],
            TimeRange::from_hours(9, 17),
        );

        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: TimeRule = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "test");
        assert_eq!(deserialized.name, "Test Rule");
        assert!(deserialized.days.contains(&Weekday::Monday));
    }

    #[test]
    fn rule_set_serialization() {
        let set = TimeRuleSet::with_defaults();
        let json = serde_json::to_string(&set).unwrap();
        let deserialized: TimeRuleSet = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.rules.len(), 2);
    }
}

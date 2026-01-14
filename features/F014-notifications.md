# F014: Desktop Notifications

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | medium | aegis-core |

## Description

Notify parents when content blocked.

## Dependencies

- **Requires**: F007
- **Blocks**: None

## Acceptance Criteria

- [x] Notify on block (not warn)
- [x] Show site and category
- [x] Rate-limit 1/min
- [x] Can disable
- [x] Cross-platform

## Implementation

### Types

```rust
// Notification settings
pub struct NotificationSettings {
    pub enabled: bool,
}

// Blocked event info
pub struct BlockedEvent {
    pub source: Option<String>,      // Site/app name
    pub category: Option<Category>,   // Content category
    pub rule_name: Option<String>,    // Rule that triggered
    pub is_time_block: bool,          // Time vs content block
}

// Send result
pub enum NotificationResult {
    Sent,
    RateLimited,
    Disabled,
    Failed(String),
}

// Manager with rate limiting
pub struct NotificationManager {
    settings: Arc<RwLock<NotificationSettings>>,
    rate_limit: Arc<RwLock<RateLimitState>>,
}
```

### Usage

```rust
use aegis_core::notifications::{NotificationManager, BlockedEvent};
use aegis_core::rule_engine::{RuleAction, RuleSource};

let manager = NotificationManager::new();

// Notify about a block event
let event = BlockedEvent::from_rule_source(&rule_source, Some("ChatGPT".to_string()));
let result = manager.notify_block(&event);

// Or use convenience method
let result = manager.notify_if_blocked(action, &source, Some("ChatGPT".to_string()));

// Enable/disable
manager.disable();
manager.enable();
```

### Features

- **Rate limiting**: 60 seconds between notifications
- **Cross-platform**: Uses `notify-rust` (Windows, macOS, Linux)
- **Optional**: Compile without notifications via feature flag
- **Thread-safe**: Arc<RwLock> for concurrent access

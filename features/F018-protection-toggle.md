# F018: Protection Toggle

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | high | aegis-core |

## Description

Allows pausing or disabling protection, with authentication required. Supports timed pauses that automatically resume.

## Dependencies

- **Requires**: F011 (System Tray), F013 (Authentication)
- **Blocks**: None

## Acceptance Criteria

- [x] Three protection states: Active, Paused, Disabled
- [x] Pause requires authentication
- [x] Timed pause (5min, 15min, 1hr) with auto-resume
- [x] Indefinite pause option
- [x] State persisted to storage
- [x] Tray icon reflects current state (via ProtectionState -> TrayStatus mapping)
- [x] Resume immediately option

## Implementation

### Files

- `crates/aegis-core/src/protection.rs` - ProtectionManager, ProtectionState, PauseDuration, ProtectionEvent
- `crates/aegis-storage/src/database.rs` - get_protection_state(), set_protection_state() methods

### Types

- `ProtectionState` - Enum with Active, Paused, Disabled variants
- `PauseDuration` - Enum with Minutes(u32), Hours(u32), Indefinite variants
- `ProtectionEvent` - State change and expiry events
- `ProtectionManager` - Thread-safe manager with auth-guarded operations
- `ProtectionError` - AuthRequired, SessionInvalid, InvalidTransition errors

### API

```rust
use aegis_core::protection::{ProtectionManager, ProtectionState, PauseDuration};
use aegis_core::auth::AuthManager;

let auth = AuthManager::new();
let manager = ProtectionManager::new();

// Check current state
let state = manager.state();

// Pause with duration (requires auth)
let session = auth.create_session();
manager.pause(PauseDuration::Minutes(15), &session, &auth)?;

// Pause indefinitely (requires auth)
manager.pause(PauseDuration::Indefinite, &session, &auth)?;

// Resume protection (no auth required)
manager.resume();

// Disable completely (requires auth)
manager.disable(&session, &auth)?;

// Enable (no auth required)
manager.enable();

// Check remaining pause time
if let Some(remaining) = manager.pause_remaining() {
    println!("Resuming in {} seconds", remaining.as_secs());
}

// Check for expired pause
if let Some(event) = manager.check_expiry() {
    // Handle auto-resume
}
```

### Storage

```rust
use aegis_storage::Database;

let db = Database::in_memory().unwrap();

// Persist state
db.set_protection_state("paused").unwrap();

// Load state
let state = db.get_protection_state().unwrap(); // Some("paused")
```

### Notes

- Pause/disable require valid session; resume/enable do not (security design)
- Timed pause auto-resumes when `state()` or `check_expiry()` is called
- ProtectionState serializes as lowercase strings ("active", "paused", "disabled")
- PauseDuration presets: FIVE_MINUTES, FIFTEEN_MINUTES, THIRTY_MINUTES, ONE_HOUR

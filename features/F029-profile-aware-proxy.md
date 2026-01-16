# F029: Profile-Aware Proxy Control

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | high | aegis-core |

## Description

Auto-enable/disable proxy based on logged-in OS user profile. Child profiles enable filtering; parent/unknown profiles disable or bypass. Handle fast user switching and log all profile changes.

## Dependencies

- **Requires**: F016, F018, F019
- **Blocks**: None

## Current State

`profile.rs`:
- `get_current_os_user()` reads USER/USERNAME env var once
- `ProfileManager::get_by_os_username()` matches profiles
- No OS user change monitoring

`protection.rs`:
- `ProtectionManager` with Active/Paused/Disabled states
- Auth-guarded pause/disable
- No connection to profile system

## Acceptance Criteria

### Profile Types

- [ ] Child profile: filtering enabled, rules applied
- [ ] Parent profile: filtering disabled (unrestricted)
- [ ] Unknown user: configurable (default: enabled with defaults)
- [ ] Profile field: `proxy_mode`: `enabled` | `disabled` | `passthrough`

### Auto-Enable on Child Login

- [ ] Detect OS user switch (polling or OS events)
- [ ] Look up profile by OS username
- [ ] If child profile: enable system proxy, apply rules
- [ ] If profile has custom rules: load into classifier
- [ ] Show tray notification: "Protection active for [Child]"

### Auto-Disable on Parent Login

- [ ] Detect parent profile login
- [ ] Disable system proxy (traffic not intercepted)
- [ ] OR passthrough mode (proxy running but not filtering)
- [ ] Show tray notification: "Protection paused (parent mode)"
- [ ] No auth required for auto-disable on user switch

### Fast User Switching

- [ ] Windows: Monitor `WTSRegisterSessionNotification`
- [ ] macOS: Monitor `NSWorkspace` session notifications
- [ ] Linux: Monitor `logind` D-Bus signals
- [ ] Handle rapid switches (debounce 500ms)
- [ ] Queue profile changes, process in order

### Per-Profile Settings

- [ ] `proxy_enabled`: bool (default true for child, false for parent)
- [ ] `filtering_level`: `strict` | `moderate` | `permissive`
- [ ] `system_proxy_control`: `auto` | `manual`
- [ ] Store in profile, persist to database

### System Proxy Management

- [ ] Enable proxy: set system proxy to 127.0.0.1:PORT
- [ ] Disable proxy: remove system proxy settings
- [ ] Passthrough: proxy runs but allows all traffic
- [ ] Handle proxy already set by another app (warn, don't overwrite)
- [ ] Restore previous proxy settings on parent login (optional)

### Logging

- [ ] Log all profile switches with timestamp
- [ ] Log: OS user, profile name, previous state, new state
- [ ] Log proxy enable/disable actions
- [ ] Log rule changes on profile switch
- [ ] Queryable in dashboard logs view

### API

- [ ] `ProfileProxyController::new(profiles, protection, proxy)`
- [ ] `ProfileProxyController::start_monitoring()` -> watch for user changes
- [ ] `ProfileProxyController::on_user_change(callback)`
- [ ] `ProfileProxyController::current_profile() -> Option<&UserProfile>`
- [ ] `ProfileProxyController::force_check()` -> manual refresh

### Edge Cases

- [ ] App starts with child logged in: enable immediately
- [ ] App starts with parent logged in: stay disabled
- [ ] Profile deleted while active: fall back to defaults
- [ ] Multiple profiles match same OS user: first match wins
- [ ] Lock screen: no change (same user session)
- [ ] Remote desktop: detect and handle appropriately

## Notes

OS user change detection:

**Windows:**
```rust
// WTS session notifications
WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);
// Handle WM_WTSSESSION_CHANGE message
```

**macOS:**
```rust
// NSWorkspace notifications
NSWorkspaceSessionDidBecomeActiveNotification
NSWorkspaceSessionDidResignActiveNotification
```

**Linux:**
```rust
// logind D-Bus
org.freedesktop.login1.Session.Lock
org.freedesktop.login1.Session.Unlock
```

Profile switch event:
```rust
pub struct ProfileSwitchEvent {
    pub timestamp: DateTime<Utc>,
    pub os_username: String,
    pub previous_profile: Option<String>,
    pub new_profile: Option<String>,
    pub proxy_action: ProxyAction, // Enabled, Disabled, Passthrough
}
```

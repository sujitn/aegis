# F011: System Tray

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | high | aegis-tray |

## Description

System tray icon with status and menu.

## Dependencies

- **Requires**: F001
- **Blocks**: F012

## Acceptance Criteria

- [x] Tray icon on startup
- [x] Status indicator (protected/paused/error)
- [x] Menu: Dashboard, Settings, Logs, Pause, Quit
- [x] Double-click opens settings
- [x] Background operation

## Implementation

- `crates/aegis-tray/src/lib.rs` - Main module exports
- `crates/aegis-tray/src/status.rs` - TrayStatus enum (Protected/Paused/Error)
- `crates/aegis-tray/src/menu.rs` - MenuAction enum and TrayMenu builder
- `crates/aegis-tray/src/icon.rs` - Dynamic shield icon generation per status
- `crates/aegis-tray/src/tray.rs` - SystemTray struct with config, events, and polling
- `crates/aegis-tray/src/error.rs` - TrayError types

### Dependencies
- `tray-icon` - Cross-platform system tray
- `muda` - Cross-platform menu
- `image` - PNG icon support

### Usage

```rust
use aegis_tray::{SystemTray, TrayConfig, TrayStatus, TrayEvent, MenuAction};

// Create tray with config
let config = TrayConfig::new()
    .with_app_name("Aegis")
    .with_initial_status(TrayStatus::Protected);

let (mut tray, rx) = SystemTray::with_config(config)?;

// Initialize (must be on main thread)
tray.init()?;

// Poll for events in event loop
for event in tray.poll_events() {
    match event {
        TrayEvent::MenuAction(MenuAction::Dashboard) => { /* open dashboard */ }
        TrayEvent::MenuAction(MenuAction::Quit) => break,
        TrayEvent::DoubleClick => { /* open settings */ }
        _ => {}
    }
}

// Update status
tray.set_status(TrayStatus::Paused)?;
```

# F028: Background Service Mode

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | high | aegis-app |

## Description

Run Aegis invisibly in background with tray icon only. No console window. Dashboard opens on demand. File-based logging with rotation and in-dashboard viewer.

## Dependencies

- **Requires**: F011, F012
- **Blocks**: None

## Current State

`crates/aegis-app/src/main.rs`:
- Console window visible (Windows shows cmd)
- `tracing_subscriber::fmt()` logs to stdout
- `run_dashboard(db)` blocks as main loop
- Tray crate exists but not integrated

## Acceptance Criteria

### No Visible Window

- [ ] Windows: `#![windows_subsystem = "windows"]` attribute
- [ ] macOS: LSUIElement=true in Info.plist (no dock icon)
- [ ] Linux: no terminal attachment
- [ ] App starts silently after login
- [ ] No splash screen

### Tray Icon Primary Interface

- [ ] Tray icon shows on startup (before dashboard)
- [ ] Status indicator: green (active), yellow (paused), red (error)
- [ ] Menu items:
  - Open Dashboard
  - Pause Protection (with duration picker)
  - Resume Protection
  - View Logs
  - Settings
  - Quit
- [ ] Double-click opens dashboard
- [ ] Tooltip shows current status and stats

### Dashboard On-Demand

- [ ] Dashboard not opened by default
- [ ] Open via tray menu or double-click
- [ ] Close dashboard returns to tray (not quit)
- [ ] Window close button minimizes to tray
- [ ] Explicit "Quit" to fully exit
- [ ] Single instance: second launch focuses existing

### File Logging

- [ ] Log to `<data_dir>/logs/aegis.log`
- [ ] Log levels: error, warn, info, debug, trace
- [ ] Default level: info (configurable)
- [ ] Include timestamp, level, target, message
- [ ] Format: `2024-01-15T10:30:45.123Z INFO aegis_proxy: Message`

### Log Rotation

- [ ] Rotate when file exceeds size (default: 10MB)
- [ ] Keep N rotated files (default: 5)
- [ ] Naming: aegis.log, aegis.1.log, aegis.2.log...
- [ ] Optional: rotate daily instead of by size
- [ ] Compress rotated files (gzip)
- [ ] Auto-delete oldest when limit reached

### Log Viewer in Dashboard

- [ ] New "Logs" view in dashboard (F012)
- [ ] Real-time log tail (auto-refresh)
- [ ] Scroll through history
- [ ] Filter by level (error, warn, info, etc.)
- [ ] Search text in logs
- [ ] Copy selected lines
- [ ] "Open log folder" button
- [ ] Clear current view (not file)

### Startup Behavior

- [ ] Command-line flags:
  - `--no-tray`: skip tray, open dashboard directly
  - `--debug`: enable debug logging
  - `--log-level <level>`: set log level
- [ ] First-run: open dashboard for setup wizard
- [ ] Subsequent: tray only (silent start)
- [ ] Auto-start on login (optional, configured in setup)

### Error Handling

- [ ] Log file write failures: fall back to stderr
- [ ] Tray init failure: fall back to dashboard mode
- [ ] Show notification on critical errors
- [ ] "Service unhealthy" tray status if proxy fails

## Notes

Windows subsystem attribute prevents console:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
```

Log file location:
- Windows: `%APPDATA%\aegis\aegis\data\logs\`
- macOS: `~/Library/Application Support/aegis/logs/`
- Linux: `~/.local/share/aegis/logs/`

Recommended crates:
- `tracing-appender`: file logging with rotation
- `tracing-subscriber`: layered subscribers (file + optional console)

Dashboard minimize vs quit:
- Window X button → minimize to tray
- File > Quit or tray Quit → full shutdown
- macOS Cmd+Q → full shutdown

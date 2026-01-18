# F012: Parent Dashboard

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | high | aegis-ui |

## Description

Desktop GUI for parents. Full control over profiles, rules, and monitoring. Password protected.

## Dependencies

- **Requires**: F008, F011, F013
- **Blocks**: F015

## Acceptance Criteria

### Access
- [x] Password required to open
- [x] Session timeout (15 min idle)
- [x] Lock button for quick re-lock

### Dashboard View (Home)
- [x] Today's summary cards:
  - Total prompts checked
  - Blocked count (red)
  - Warnings count (yellow)
  - Allowed count (green)
- [x] Current status: Active/Paused/Disabled
- [x] Current profile indicator
- [x] Current mode: Extension/Proxy
- [x] Quick actions:
  - Pause (15min/1hr/Until Tomorrow)
  - Switch Profile dropdown
  - Open Logs
- [x] Recent activity feed (last 10 events)
- [ ] Weekly trend chart (optional - future enhancement)

### Profiles View
- [x] List all profiles with status
- [x] Create new profile:
  - Name
  - OS username (autocomplete from system)
  - Protection level preset
- [x] Edit profile
- [x] Delete profile (confirm dialog)
- [x] Enable/disable per profile
- [ ] Duplicate profile (future enhancement)

### Rules View (per profile)
- [x] Tab: Time Rules
  - List rules with enable toggle
  - Add/edit rule (days, start, end)
  - Presets: Bedtime, School Hours
- [x] Tab: Content Rules
  - List categories with action dropdown
  - Threshold slider
  - Enable/disable per category

### Logs View
- [x] Table: timestamp, profile, site, action, category, preview
- [x] Search box
- [x] Filter by: profile, action, category, date range
- [x] Export to CSV
- [x] Clear logs (confirm + password)

### Settings View
- [x] Change password
- [x] Mode selection (Extension/Proxy)
  - If switching to Proxy: CA install wizard
  - If switching to Extension: extension install prompt
- [ ] Notification preferences (future - F014)
- [x] Check for updates
- [x] About (version, links)
- [ ] Uninstall button → F020

### Navigation
- [x] Sidebar: Dashboard, Profiles, Logs, Settings
- [x] Header: Current profile, status indicator, lock button
- [x] Footer: Version, mode indicator

## Implementation

- `crates/aegis-ui/src/lib.rs` - Main App component with Dioxus integration
- `crates/aegis-ui/src/state.rs` - AppState for application data management
- `crates/aegis-ui/src/error.rs` - UiError types
- `crates/aegis-ui/src/views/login.rs` - Login/password setup screen
- `crates/aegis-ui/src/views/dashboard.rs` - Dashboard home with stats cards
- `crates/aegis-ui/src/views/profiles.rs` - Profile CRUD with editor dialog
- `crates/aegis-ui/src/views/rules.rs` - Time/content rules tabs
- `crates/aegis-ui/src/views/logs.rs` - Activity logs with filtering
- `crates/aegis-ui/src/views/settings.rs` - Settings with password change

### Dependencies
- `dioxus` - Rust GUI framework with desktop support
- `csv` - CSV export functionality
- `directories` - User directories for export

### Usage

```rust
use aegis_ui::run_dashboard;
use aegis_storage::Database;

// Create database
let db = Database::new().expect("Failed to open database");

// Run the dashboard
run_dashboard(db).expect("Failed to run dashboard");
```

## Notes

Framework: Dioxus (native desktop). Design: clean, minimal, parent-friendly. Colors: green (safe), yellow (warn), red (block/error).

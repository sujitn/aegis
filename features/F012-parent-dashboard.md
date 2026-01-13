# F012: Parent Dashboard

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | high | aegis-ui |

## Description

Desktop GUI for parents. Full control over profiles, rules, and monitoring. Password protected.

## Dependencies

- **Requires**: F008, F011, F013
- **Blocks**: F015

## Acceptance Criteria

### Access
- [ ] Password required to open
- [ ] Session timeout (15 min idle)
- [ ] Lock button for quick re-lock

### Dashboard View (Home)
- [ ] Today's summary cards:
  - Total prompts checked
  - Blocked count (red)
  - Warnings count (yellow)
  - Allowed count (green)
- [ ] Current status: Active/Paused/Disabled
- [ ] Current profile indicator
- [ ] Current mode: Extension/Proxy
- [ ] Quick actions:
  - Pause (15min/1hr/Until Tomorrow)
  - Switch Profile dropdown
  - Open Logs
- [ ] Recent activity feed (last 10 events)
- [ ] Weekly trend chart (optional)

### Profiles View
- [ ] List all profiles with status
- [ ] Create new profile:
  - Name
  - OS username (autocomplete from system)
  - Protection level preset
- [ ] Edit profile
- [ ] Delete profile (confirm dialog)
- [ ] Enable/disable per profile
- [ ] Duplicate profile

### Rules View (per profile)
- [ ] Tab: Time Rules
  - List rules with enable toggle
  - Add/edit rule (days, start, end)
  - Presets: Bedtime, School Hours
- [ ] Tab: Content Rules
  - List categories with action dropdown
  - Threshold slider
  - Enable/disable per category

### Logs View
- [ ] Table: timestamp, profile, site, action, category, preview
- [ ] Search box
- [ ] Filter by: profile, action, category, date range
- [ ] Export to CSV
- [ ] Clear logs (confirm + password)

### Settings View
- [ ] Change password
- [ ] Mode selection (Extension/Proxy)
  - If switching to Proxy: CA install wizard
  - If switching to Extension: extension install prompt
- [ ] Notification preferences
- [ ] Check for updates
- [ ] About (version, links)
- [ ] Uninstall button → F020

### Navigation
- [ ] Sidebar: Dashboard, Profiles, Logs, Settings
- [ ] Header: Current profile, status indicator, lock button
- [ ] Footer: Version, mode indicator

## Notes

Framework: egui (native) or Tauri (web-based). Design: clean, minimal, parent-friendly. Colors: green (safe), yellow (warn), red (block/error).

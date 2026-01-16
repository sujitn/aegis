# F023: Auto Update

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | medium | aegis-app |

## Description

Check for updates and prompt user. Optional auto-download.

## Dependencies

- **Requires**: F021, F022
- **Blocks**: None

## Acceptance Criteria

- [ ] Check GitHub Releases for new version
- [ ] Compare semver
- [ ] Notify via tray/dashboard when update available
- [ ] Download update in background
- [ ] Prompt to install (requires password)
- [ ] Preserve config/data during update
- [ ] Changelog display

## Notes

Check interval: daily. Use GitHub Releases API. Optional: Sparkle (macOS), WinSparkle (Windows).

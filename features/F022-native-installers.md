# F022: Native Installers

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | high | infrastructure |

## Description

Platform-native installers with guided setup. One-click install experience.

## Dependencies

- **Requires**: F021
- **Blocks**: None

## Acceptance Criteria

### macOS
- [ ] .dmg installer with drag-to-Applications
- [ ] .pkg installer (optional, for managed deploy)
- [ ] Notarized and signed
- [ ] Auto-register for Login Items (optional)
- [ ] Gatekeeper compatible

### Windows
- [ ] .msi installer (WiX or similar)
- [ ] .exe installer (NSIS optional)
- [ ] Signed with EV certificate
- [ ] Add to Start Menu
- [ ] Optional: add to startup
- [ ] SmartScreen compatible

### Linux
- [ ] .deb package (Debian/Ubuntu)
- [ ] .rpm package (Fedora/RHEL)
- [ ] .AppImage (universal)
- [ ] Desktop entry
- [ ] Systemd service (optional)

## Install Flow

```
Download → Run Installer → First-Run Setup (F015) → Ready
```

## Notes

Tools: cargo-bundle, cargo-wix, cargo-deb, cargo-rpm, appimagetool. Installers trigger first-run wizard on initial launch.

# F030: Autostart & Persistence

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | high | aegis-app |

## Description

Start Aegis automatically on system boot/login. Configurable in settings, enabled by default, starts minimized to tray. Clean removal on uninstall. Parent can lock the setting to prevent tampering.

## Dependencies

- **Requires**: F013, F020, F028
- **Blocks**: None

## Acceptance Criteria

### Start on Boot/Login

- [ ] Windows: Registry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
- [ ] macOS: LaunchAgent plist in `~/Library/LaunchAgents/`
- [ ] Linux: `.desktop` file in `~/.config/autostart/`
- [ ] Start after user login (not system boot for per-user install)
- [ ] Handle both per-user and system-wide installs

### Enable/Disable in Settings

- [ ] Toggle in Settings > General > "Start on login"
- [ ] Immediate effect (no restart required)
- [ ] Show current state accurately
- [ ] Verify registration succeeded (check registry/plist)

### Enabled by Default

- [ ] First-run setup enables autostart
- [ ] Checkbox in setup wizard: "Start Aegis when I log in" (checked)
- [ ] User can uncheck during setup
- [ ] If skipped, default to enabled

### Start Minimized to Tray

- [ ] No dashboard window on autostart
- [ ] Tray icon appears immediately
- [ ] Proxy starts in background
- [ ] First-run exception: show setup wizard
- [ ] Command-line flag: `--minimized` (set in autostart entry)

### Removed on Uninstall

- [ ] Uninstaller removes autostart entry
- [ ] Windows: delete registry key
- [ ] macOS: delete LaunchAgent plist
- [ ] Linux: delete .desktop file
- [ ] Clean uninstall leaves no autostart remnants
- [ ] Handle manual uninstall (detect missing binary)

### Parent Lock Setting

- [ ] "Lock autostart setting" option in Settings
- [ ] Requires parent authentication to change
- [ ] When locked: toggle disabled, shows lock icon
- [ ] Prevents child from disabling autostart
- [ ] Store lock state in encrypted config

### Platform Implementation

#### Windows
- [ ] Registry key: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
- [ ] Value name: `Aegis`
- [ ] Value data: `"C:\Program Files\Aegis\aegis.exe" --minimized`
- [ ] Alternative: Task Scheduler for elevated start

#### macOS
- [ ] LaunchAgent: `~/Library/LaunchAgents/com.aegis.agent.plist`
- [ ] `RunAtLoad`: true
- [ ] `ProgramArguments`: ["/Applications/Aegis.app/Contents/MacOS/aegis", "--minimized"]
- [ ] `KeepAlive`: false (don't restart on crash)

#### Linux
- [ ] Desktop entry: `~/.config/autostart/aegis.desktop`
- [ ] `Exec=aegis --minimized`
- [ ] `X-GNOME-Autostart-enabled=true`
- [ ] Handle XDG autostart spec

### Error Handling

- [ ] Permission denied: show error, suggest fix
- [ ] Registry/plist corrupted: recreate entry
- [ ] Binary moved: update path automatically
- [ ] Antivirus blocks: detect and warn user

### API

- [ ] `Autostart::is_enabled() -> bool`
- [ ] `Autostart::enable() -> Result<()>`
- [ ] `Autostart::disable() -> Result<()>`
- [ ] `Autostart::is_locked() -> bool`
- [ ] `Autostart::set_locked(locked: bool, auth: &Session) -> Result<()>`
- [ ] `Autostart::get_command() -> PathBuf`

## Notes

Windows registry example:
```
[HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run]
"Aegis"="\"C:\\Program Files\\Aegis\\aegis.exe\" --minimized"
```

macOS LaunchAgent plist:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "...">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.aegis.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Applications/Aegis.app/Contents/MacOS/aegis</string>
        <string>--minimized</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
```

Linux desktop entry:
```ini
[Desktop Entry]
Type=Application
Name=Aegis
Exec=/usr/bin/aegis --minimized
Hidden=false
X-GNOME-Autostart-enabled=true
```

Recommended crate: `auto-launch` (cross-platform autostart)

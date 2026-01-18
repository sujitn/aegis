# F024: Extension Deployment

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | high | extension, aegis-core, aegis-ui |

## Description

Deploy the Aegis browser extension with easy installation options. Supports two deployment modes:

1. **First Iteration (No Store)**: Auto-install via native app using OS policies, or manual developer mode installation
2. **Future (Chrome Web Store)**: One-click installation from Chrome Web Store

## Dependencies

- **Requires**: F010 (Browser Extension)
- **Blocks**: None

## Acceptance Criteria

### Phase 1: Easy Install Without Store (Current Priority)

#### Auto-Install via Native App
- [x] Windows: Registry-based extension installation
- [x] macOS: Chrome policies JSON file
- [x] Linux: Chrome policies JSON file
- [x] Extension ID generated and consistent
- [x] Settings UI offers "Auto Install" button

#### Manual Installation Support
- [x] Clear instructions in README and app
- [x] Pre-packaged CRX file in releases
- [x] Settings UI shows manual steps if auto-install fails

### Phase 2: Chrome Web Store (Future)

- [ ] Chrome Web Store developer account created
- [ ] Extension submitted and approved
- [ ] App links to store for one-click install

## Implementation

### Phase 1: Auto-Install Without Store

#### How Chrome Extension Policies Work

Chrome supports enterprise deployment of extensions via:
- **Windows**: Registry keys under `HKLM\SOFTWARE\Policies\Google\Chrome`
- **macOS/Linux**: JSON file in Chrome's managed policies directory

For **unpacked extensions** (developer mode), we use:
- **Windows**: `HKLM\SOFTWARE\Policies\Google\Chrome\ExtensionInstallSources`
- **All OS**: External extensions JSON file

#### Windows Implementation

```rust
// In aegis-core/src/extension_install.rs

use winreg::enums::*;
use winreg::RegKey;

const EXTENSION_ID: &str = "aegis-extension"; // Will be replaced with actual ID

/// Install extension via Windows Registry (requires admin)
pub fn install_extension_windows(extension_path: &Path) -> Result<(), String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    // Create Chrome policies key
    let (key, _) = hklm.create_subkey(
        r"SOFTWARE\Policies\Google\Chrome\ExtensionSettings"
    ).map_err(|e| e.to_string())?;

    // Allow extension from local path
    let extension_json = format!(r#"{{
        "installation_mode": "allowed",
        "override_update_url": true
    }}"#);

    key.set_value(EXTENSION_ID, &extension_json)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Alternative: Use external_extensions.json
pub fn install_extension_external_json(extension_path: &Path) -> Result<(), String> {
    // Chrome looks for external extensions in:
    // Windows: %LOCALAPPDATA%\Google\Chrome\User Data\Default\External Extensions\
    // This method works without admin rights

    let local_app_data = std::env::var("LOCALAPPDATA")
        .map_err(|e| e.to_string())?;

    let external_ext_dir = PathBuf::from(local_app_data)
        .join("Google")
        .join("Chrome")
        .join("User Data")
        .join("Default")
        .join("External Extensions");

    std::fs::create_dir_all(&external_ext_dir)
        .map_err(|e| e.to_string())?;

    let json_path = external_ext_dir.join(format!("{}.json", EXTENSION_ID));
    let json_content = format!(r#"{{
        "external_crx": "{}",
        "external_version": "1.0.0"
    }}"#, extension_path.display());

    std::fs::write(json_path, json_content)
        .map_err(|e| e.to_string())?;

    Ok(())
}
```

#### macOS Implementation

```rust
// Chrome policies location: /Library/Google/Chrome/managed_preferences/

pub fn install_extension_macos(extension_path: &Path) -> Result<(), String> {
    let policies_dir = PathBuf::from("/Library/Google/Chrome/managed_preferences");

    std::fs::create_dir_all(&policies_dir)
        .map_err(|e| e.to_string())?;

    let plist_path = policies_dir.join("com.google.Chrome.plist");

    // Create plist with extension settings
    let plist_content = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>ExtensionInstallAllowlist</key>
    <array>
        <string>{}</string>
    </array>
</dict>
</plist>"#, EXTENSION_ID);

    std::fs::write(plist_path, plist_content)
        .map_err(|e| e.to_string())?;

    Ok(())
}
```

#### Linux Implementation

```rust
// Chrome policies location: /etc/opt/chrome/policies/managed/

pub fn install_extension_linux(extension_path: &Path) -> Result<(), String> {
    let policies_dir = PathBuf::from("/etc/opt/chrome/policies/managed");

    std::fs::create_dir_all(&policies_dir)
        .map_err(|e| e.to_string())?;

    let json_path = policies_dir.join("aegis-extension.json");

    let json_content = format!(r#"{{
        "ExtensionInstallAllowlist": ["{}"]
    }}"#, EXTENSION_ID);

    std::fs::write(json_path, json_content)
        .map_err(|e| e.to_string())?;

    Ok(())
}
```

### Setup Wizard Integration

The setup wizard (F015) includes extension installation in the interception mode step.
See `crates/aegis-ui/src/views/setup.rs` for the Dioxus implementation.

### Bundling Extension with App

The extension should be bundled with the native app installer:

```
Aegis.app/
├── Contents/
│   ├── MacOS/aegis
│   └── Resources/
│       └── extension/          # Bundled extension
│           ├── manifest.json
│           ├── dist/
│           └── ...

# Windows MSI
C:\Program Files\Aegis\
├── aegis.exe
└── extension\                  # Bundled extension
    ├── manifest.json
    ├── dist\
    └── ...
```

### Phase 2: Chrome Web Store (Future)

When ready for Chrome Web Store:

1. Create developer account ($5 registration)
2. Prepare store assets (icons, screenshots)
3. Submit extension for review
4. Update app to link to store instead of auto-install

Store listing details remain the same as previously documented.

## Manual Installation Instructions

For users who need to install manually:

### Chrome / Edge

1. Download the extension from the Aegis releases page
2. Extract the ZIP file to a folder
3. Open Chrome and navigate to `chrome://extensions`
4. Enable "Developer mode" (toggle in top-right corner)
5. Click "Load unpacked"
6. Select the extracted extension folder
7. The Aegis extension icon should appear in your toolbar

### Keeping Extension Updated

When using manual installation:
- Extension will NOT auto-update
- Check Aegis releases for new extension versions
- Re-install by loading the new unpacked version

## Notes

### Why Auto-Install?

For parental controls, we need reliable installation that:
- Children cannot easily disable or remove
- Parents don't need technical knowledge to set up
- Works even if Chrome Web Store is blocked

### Security Considerations

- Auto-installed extensions require admin/root privileges
- This is appropriate for parental control software
- Users are informed during Aegis installation

### Browser Support

| Browser | Auto-Install | Manual Install |
|---------|-------------|----------------|
| Chrome | Yes (policies) | Yes |
| Edge | Yes (same as Chrome) | Yes |
| Brave | Yes (Chromium policies) | Yes |
| Firefox | No (different system) | Planned (F025) |
| Safari | No | Not supported |

### Testing Auto-Install

```bash
# Windows (PowerShell as Admin)
# Check if policy was applied
reg query "HKLM\SOFTWARE\Policies\Google\Chrome" /s

# macOS
# Check managed preferences
defaults read /Library/Managed\ Preferences/com.google.Chrome

# Linux
# Check policies
cat /etc/opt/chrome/policies/managed/*.json
```

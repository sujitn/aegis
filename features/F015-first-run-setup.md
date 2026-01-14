# F015: First-Run Setup

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | high | aegis-ui |

## Description

Setup wizard for first launch. Guides new users through initial configuration including password setup, protection level selection, interception mode choice, CA certificate installation (for proxy mode), and first profile creation.

## Dependencies

- **Requires**: F012 (Parent Dashboard), F013 (Authentication), F017 (Interception Mode), F019 (User Profiles)
- **Blocks**: None

## Acceptance Criteria

- [x] Detect first run (via `is_auth_setup()`)
- [x] Password creation + confirm (min 6 chars, strength indicator)
- [x] Protection level choice (Standard/Strict/Custom with default rules)
- [x] Generate root CA certificate path setup
- [x] Guide user through CA trust installation (OS-specific instructions)
- [x] Configure interception mode (Extension/Proxy) with mode selection
- [x] Create default rules (time rules + content rules per protection level)

## API

```rust
pub enum SetupStep {
    Welcome,
    Password,
    ProtectionLevel,
    InterceptionMode,
    CaInstall,
    Profile,
    Complete,
}

pub enum ProtectionLevel {
    Standard,  // Block harmful, school night bedtime
    Strict,    // Lower thresholds, early bedtime
    Custom,    // No default rules
}

pub struct SetupWizardState {
    pub step: SetupStep,
    pub password: String,
    pub confirm_password: String,
    pub protection_level: ProtectionLevel,
    pub interception_mode: SetupInterceptionMode,
    pub profile_name: String,
    pub profile_os_username: String,
    pub ca_generated: bool,
    pub ca_cert_path: Option<String>,
    pub error: Option<String>,
}
```

## Steps

1. **Welcome** - Introduction to Aegis features and protection
2. **Password** - Create parent password (min 6 chars) with strength indicator
3. **Protection Level** - Choose Standard/Strict/Custom preset
4. **Interception Mode** - Choose Browser Extension (recommended) or System Proxy
5. **CA Install** - (Proxy mode only) Generate CA path and show OS-specific install instructions
6. **Profile** - Create first child profile with name and optional OS username
7. **Complete** - Summary and next steps

## Notes

- Extension mode skips CA installation step
- First run detection uses `is_auth_setup()` from database
- Protection levels create appropriate default time/content rules
- OS-specific CA installation instructions for Windows, macOS, and Linux
- Wizard state is separate from app state for clean separation

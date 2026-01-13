# F015: First-Run Setup

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | high | aegis-ui |

## Description

Setup wizard for first launch. Includes CA installation for MITM proxy.

## Dependencies

- **Requires**: F012, F013
- **Blocks**: None

## Acceptance Criteria

- [ ] Detect first run
- [ ] Password creation + confirm
- [ ] Protection level choice (Standard/Strict/Custom)
- [ ] Generate root CA certificate
- [ ] Guide user through CA trust installation
- [ ] Configure system proxy settings
- [ ] Create default rules

## Notes

Steps: Welcome, Password, Protection, CA Install, Proxy Setup, Complete

CA install per OS:
- macOS: security add-trusted-cert or Keychain Access
- Windows: certutil or Certificate Manager
- Linux: update-ca-certificates

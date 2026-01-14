# F017: Interception Mode

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | high | aegis-core |

## Description

Allows switching between Browser Extension mode and MITM Proxy mode for intercepting LLM traffic. Each mode has trade-offs:

| Mode | Coverage | Setup | Best For |
|------|----------|-------|----------|
| Extension | Browser only | Install extension | Simple setup, corporate devices |
| Proxy | All apps | Install CA cert | Full protection, home devices |

## Dependencies

- **Requires**: F010 (Browser Extension), F016 (MITM Proxy)
- **Blocks**: F015 (First-Run Setup)

## Acceptance Criteria

- [x] InterceptionMode enum with Extension and Proxy variants
- [x] InterceptionManager for thread-safe mode state
- [x] Mode persisted to storage (config key-value via restore_mode)
- [x] Mode change requires parent authentication
- [x] Events emitted on mode change
- [x] Mode descriptions for UI display
- [x] Default mode is Extension (simpler setup)

## API

```rust
pub enum InterceptionMode {
    Extension,
    Proxy,
}

pub struct InterceptionManager {
    pub fn mode(&self) -> InterceptionMode;
    pub fn set_mode(&self, mode: InterceptionMode, session: &SessionToken, auth: &AuthManager) -> Result<InterceptionEvent>;
}
```

## Notes

- Mode switching is a parent-only operation (requires auth)
- Switching to Proxy mode should warn about CA certificate installation
- The actual proxy start/stop is handled by aegis-app, not this module

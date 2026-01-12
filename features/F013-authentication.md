# F013: Parent Authentication

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | critical | aegis-core |

## Description

Password protection for settings and rules.

## Dependencies

- **Requires**: F008
- **Blocks**: F012, F015

## Acceptance Criteria

- [ ] Set password on first run
- [ ] Required for settings
- [ ] Required for API rule changes
- [ ] Argon2 hashing
- [ ] Min 6 characters
- [ ] Session timeout 15min

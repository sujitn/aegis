# F013: Parent Authentication

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | critical | aegis-core |

## Description

Password protection for settings and rules.

## Dependencies

- **Requires**: F008
- **Blocks**: F012, F015

## Acceptance Criteria

- [x] Set password on first run
- [x] Required for settings
- [x] Required for API rule changes
- [x] Argon2 hashing
- [x] Min 6 characters
- [x] Session timeout 15min

## Implementation

- `AuthManager` - Main authentication manager combining hashing and sessions
- `SessionToken` - Opaque session token for authenticated users
- `SessionManager` - Thread-safe session management with automatic expiry
- `AuthError` - Comprehensive error types for auth failures
- Password validation enforces minimum 6 characters
- Argon2id hashing with random salts via `rand` crate
- Session timeout of 15 minutes with refresh on use

# F019: User Profiles

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | critical | aegis-core |

## Description

Per-child user profiles that map OS usernames to time and content rules.

## Dependencies

- **Requires**: F005 (Time Rules), F006 (Content Rules), F008 (SQLite Storage)
- **Blocks**: F015

## Acceptance Criteria

- [x] Profile has name, OS username, time rules, content rules
- [x] Auto-detect OS user to load correct profile
- [x] No profile = unrestricted (parent mode)
- [x] CRUD operations for profiles
- [x] Default presets (child-safe profile)
- [x] Serialization support for persistence

## Implementation

- `UserProfile` - Profile with name, os_username, time_rules, content_rules, enabled flag
- `ProfileManager` - In-memory profile management with OS user lookup (case-insensitive)
- `ProfileRepo` - Database persistence in aegis-storage with schema v2 migration
- `get_current_os_user()` - Platform-specific OS username detection (USER/USERNAME env vars)
- `with_child_defaults()` - Family-safe preset with bedtime rules and content filtering
- `unrestricted()` - Parent mode preset with no rules

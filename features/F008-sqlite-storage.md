# F008: SQLite Storage

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | critical | aegis-storage |

## Description

Database for events, rules, config. Privacy-preserving.

## Dependencies

- **Requires**: F001
- **Blocks**: F009, F012, F013

## Acceptance Criteria

- [x] Create DB in app data directory
- [x] Migrations on startup
- [x] Store events (hash + preview, not full prompt)
- [x] Store rules as JSON
- [x] Store password hash
- [x] Connection pooling
- [x] Daily stats aggregation

## Notes

Tables: events, daily_stats, rules, config, auth

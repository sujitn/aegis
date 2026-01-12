# F008: SQLite Storage

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | critical | aegis-storage |

## Description

Database for events, rules, config. Privacy-preserving.

## Dependencies

- **Requires**: F001
- **Blocks**: F009, F012, F013

## Acceptance Criteria

- [ ] Create DB in app data directory
- [ ] Migrations on startup
- [ ] Store events (hash + preview, not full prompt)
- [ ] Store rules as JSON
- [ ] Store password hash
- [ ] Connection pooling
- [ ] Daily stats aggregation

## Notes

Tables: events, daily_stats, rules, config, auth

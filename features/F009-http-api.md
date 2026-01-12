# F009: HTTP API Server

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | critical | aegis-server |

## Description

Local HTTP server for browser extension. Localhost only.

## Dependencies

- **Requires**: F007, F008
- **Blocks**: F010

## Acceptance Criteria

- [ ] Bind 127.0.0.1:8765
- [ ] CORS for extension
- [ ] POST /api/check (<100ms)
- [ ] GET /api/stats
- [ ] GET /api/logs
- [ ] GET/PUT /api/rules (auth for PUT)
- [ ] POST /api/auth/verify

## Notes

Response: action (allow/block/warn), reason, categories, latency_ms

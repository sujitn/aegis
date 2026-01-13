# F009: HTTP API Server

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | critical | aegis-server |

## Description

Local HTTP server for browser extension. Localhost only.

## Dependencies

- **Requires**: F007, F008
- **Blocks**: F010

## Acceptance Criteria

- [x] Bind 127.0.0.1:8765
- [x] CORS for extension
- [x] POST /api/check (<100ms)
- [x] GET /api/stats
- [x] GET /api/logs
- [x] GET/PUT /api/rules (auth for PUT)
- [x] POST /api/auth/verify

## Implementation

- `Server` - Main HTTP server struct with configurable host/port
- `ServerConfig` - Configuration (host, port, db_path)
- `AppState` - Shared state (Database, AuthManager, TieredClassifier, RuleEngine, ProfileManager)
- `handlers` - Route handlers for all endpoints
- `error` - API error types with proper HTTP status codes
- `models` - Request/response types with serde serialization

### Endpoints

| Method | Path | Description | Auth |
|--------|------|-------------|------|
| POST | /api/check | Classify prompt, return action | No |
| GET | /api/stats | Get aggregated statistics | No |
| GET | /api/logs | Get event logs (paginated) | No |
| GET | /api/rules | Get all rules | No |
| PUT | /api/rules | Update rules | Yes |
| POST | /api/auth/verify | Verify password, get session | No |

### Response Format

```json
{
  "action": "allow|warn|block",
  "reason": "rule name or 'allowed'",
  "categories": [{"category": "violence", "confidence": 0.95, "tier": "keyword"}],
  "latency_ms": 1
}
```

## Notes

Response: action (allow/block/warn), reason, categories, latency_ms

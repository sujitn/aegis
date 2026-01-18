# F032: Centralized State Management

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | critical | storage, server, proxy, ui |

## Description

Implement a centralized state management system using the database as the single source of truth for application state. This addresses the current architecture problem where the dashboard runs as a subprocess and cannot share in-memory state (Arc/RwLock) with the main process containing the proxy.

### Problem Statement

Current architecture has state fragmentation:
- **Main Process**: HTTP Server + MITM Proxy share `FilteringState` via Arc (works)
- **Dashboard Subprocess**: Has its own `AppState` with no connection to proxy's `FilteringState`
- **AuthManager**: Duplicated instances with incompatible session tokens
- **ProtectionManager**: Local to UI, doesn't reflect actual proxy state

When user clicks "Pause" in dashboard:
1. Dashboard updates local `ProtectionManager` ✓
2. Dashboard calls API `/api/protection/pause` ✓
3. API validates session token ✗ (different AuthManager instance)
4. Even if auth bypassed, no way to verify state actually changed

### Solution

Use database as central state store with:
1. **Persistent state tables** for protection status, sessions, and configuration
2. **In-memory caching** with TTL for performance (proxy checks ~1000s requests/sec)
3. **Change notification** via polling with sequence numbers for cache invalidation
4. **API endpoints** for state mutations with immediate DB writes

## Dependencies

- **Requires**: F008 (SQLite Storage), F013 (Authentication), F018 (Protection Toggle)
- **Blocks**: None (enhancement to existing features)

## Architecture

### State Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           SQLite Database                                │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────────────────┐ │
│  │ app_state      │  │ sessions       │  │ state_changes              │ │
│  │ • protection   │  │ • token        │  │ • seq (auto-increment)     │ │
│  │ • pause_until  │  │ • created_at   │  │ • state_key                │ │
│  │ • updated_at   │  │ • expires_at   │  │ • changed_at               │ │
│  └────────────────┘  └────────────────┘  └────────────────────────────┘ │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
        ▼                      ▼                      ▼
┌───────────────┐      ┌───────────────┐      ┌───────────────┐
│  HTTP Server  │      │  MITM Proxy   │      │   Dashboard   │
│               │      │               │      │  (subprocess) │
│ StateManager  │      │ StateCache    │      │ StateManager  │
│ • read/write  │      │ • read-only   │      │ • read/write  │
│ • notify      │      │ • 100ms cache │      │ • poll 500ms  │
└───────────────┘      └───────────────┘      └───────────────┘
```

### Components

#### 1. Database Schema (`aegis-storage`)

```sql
-- Central application state (singleton rows)
CREATE TABLE app_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,           -- JSON encoded
    updated_at TEXT NOT NULL,      -- ISO 8601 timestamp
    updated_by TEXT                -- 'server', 'dashboard', 'tray'
);

-- Initial rows:
-- key: 'protection', value: '{"status":"active","pause_until":null}'
-- key: 'interception_mode', value: '{"mode":"proxy"}'

-- Session tokens (replaces in-memory HashMap)
CREATE TABLE sessions (
    token TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    last_used_at TEXT NOT NULL
);

CREATE INDEX idx_sessions_expires ON sessions(expires_at);

-- State change log for cache invalidation
CREATE TABLE state_changes (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    state_key TEXT NOT NULL,
    changed_at TEXT NOT NULL
);

CREATE INDEX idx_state_changes_seq ON state_changes(seq);
```

#### 2. StateManager (`aegis-core`)

```rust
/// Central state manager that reads/writes to database.
/// Used by server and dashboard for mutations.
pub struct StateManager {
    db: Arc<Database>,
    /// Local sequence number for change detection
    last_seq: AtomicI64,
}

impl StateManager {
    /// Get current protection status.
    pub fn get_protection_status(&self) -> Result<ProtectionStatus>;

    /// Pause protection until specified time (or indefinitely).
    pub fn pause_protection(&self, until: Option<DateTime<Utc>>) -> Result<()>;

    /// Resume protection immediately.
    pub fn resume_protection(&self) -> Result<()>;

    /// Disable protection completely (requires re-enable).
    pub fn disable_protection(&self) -> Result<()>;

    /// Check if there are state changes since last check.
    /// Returns new sequence number if changes detected.
    pub fn poll_changes(&self) -> Result<Option<i64>>;

    /// Subscribe to state changes (returns receiver).
    pub fn subscribe(&self, key: &str) -> Receiver<StateChange>;
}
```

#### 3. StateCache (`aegis-proxy`)

```rust
/// High-performance read-only cache for proxy.
/// Polls database every 100ms for changes.
pub struct StateCache {
    db: Arc<Database>,
    /// Cached protection status
    protection: AtomicU8,  // 0=active, 1=paused, 2=disabled
    /// Cached pause expiry (unix timestamp, 0 = no expiry)
    pause_until: AtomicI64,
    /// Last known sequence number
    last_seq: AtomicI64,
    /// Cache refresh interval
    refresh_interval: Duration,
}

impl StateCache {
    /// Fast check if filtering is enabled (atomic read, no DB).
    #[inline]
    pub fn is_filtering_enabled(&self) -> bool {
        let status = self.protection.load(Ordering::Relaxed);
        if status != 0 {
            // Check if pause expired
            let until = self.pause_until.load(Ordering::Relaxed);
            if until > 0 && Utc::now().timestamp() > until {
                // Pause expired - will be corrected on next refresh
                return true;
            }
            return false;
        }
        true
    }

    /// Background task to refresh cache from database.
    pub async fn start_refresh_task(&self);

    /// Force immediate cache refresh.
    pub fn refresh_now(&self) -> Result<()>;
}
```

#### 4. Session Storage (`aegis-storage`)

```rust
impl Database {
    /// Create a new session token with expiry.
    pub fn create_session(&self, token: &str, expires_in: Duration) -> Result<()>;

    /// Validate session token (checks expiry, updates last_used).
    pub fn validate_session(&self, token: &str) -> Result<bool>;

    /// Invalidate session token.
    pub fn invalidate_session(&self, token: &str) -> Result<()>;

    /// Clean up expired sessions.
    pub fn cleanup_expired_sessions(&self) -> Result<u64>;
}
```

## Caching Strategy

### Proxy Cache (Hot Path)

The proxy handles high-frequency requests and needs sub-millisecond state checks:

| Operation | Strategy | Latency |
|-----------|----------|---------|
| `is_filtering_enabled()` | Atomic read from cache | ~10ns |
| Cache refresh | Background poll every 100ms | N/A (async) |
| Pause expiry check | Atomic timestamp compare | ~10ns |
| Rule changes | Via existing FilteringState | ~1ms |

**Cache Invalidation:**
- Poll `state_changes` table for new sequence numbers
- On change detected, refresh affected cache entries
- Pause expiry handled inline (compare timestamps)

### Dashboard Cache (UI Responsiveness)

Dashboard needs responsive UI without hammering database:

| Operation | Strategy | Latency |
|-----------|----------|---------|
| Read protection status | Poll every 500ms | ~1ms |
| Write protection status | Immediate DB write + notify | ~5ms |
| Session validation | DB lookup with cache | ~1ms |

**Optimistic UI Updates:**
- Update local UI state immediately on user action
- Write to database in background
- Revert UI if database write fails

## Change Notification

### Sequence Number Based Polling

```rust
// Writer (dashboard/server) - on state change:
fn write_state_change(db: &Database, key: &str) -> Result<i64> {
    db.execute(
        "INSERT INTO state_changes (state_key, changed_at) VALUES (?, ?)",
        params![key, Utc::now().to_rfc3339()]
    )?;
    db.last_insert_rowid()
}

// Reader (proxy) - poll for changes:
fn poll_changes(db: &Database, last_seq: i64) -> Result<Vec<StateChange>> {
    db.query(
        "SELECT seq, state_key, changed_at FROM state_changes WHERE seq > ? ORDER BY seq",
        params![last_seq]
    )
}
```

### Notification Flow

```
1. Dashboard: User clicks "Pause 15 minutes"
   │
   ▼
2. Dashboard: StateManager.pause_protection(now + 15min)
   │
   ├─► UPDATE app_state SET value = '{"status":"paused","pause_until":"..."}' WHERE key = 'protection'
   │
   └─► INSERT INTO state_changes (state_key, changed_at) VALUES ('protection', '...')
   │
   ▼
3. Dashboard: UI shows "Paused" immediately (optimistic)

4. Proxy: StateCache background task (every 100ms)
   │
   ├─► SELECT MAX(seq) FROM state_changes WHERE seq > {last_seq}
   │
   └─► If new seq: SELECT value FROM app_state WHERE key = 'protection'
   │
   ▼
5. Proxy: Updates atomic cache values
   │
   ▼
6. Proxy: Next request sees is_filtering_enabled() = false
```

### Alternative: SQLite Write-Ahead Log Notification

For even lower latency (optional enhancement):

```rust
// Using rusqlite update_hook for instant notification
db.update_hook(Some(|action, db_name, table_name, rowid| {
    if table_name == "app_state" {
        notify_state_change(rowid);
    }
}));
```

**Note:** This only works within the same process. Cross-process notification still requires polling.

## API Endpoints

### Protection Control

```
GET  /api/protection/status
Response: { "status": "active"|"paused"|"disabled", "pause_until": "ISO8601"|null }

POST /api/protection/pause
Body: { "duration_minutes": 15 }  // or "indefinite": true
Response: { "success": true, "status": "paused", "pause_until": "ISO8601" }

POST /api/protection/resume
Body: {}
Response: { "success": true, "status": "active" }

POST /api/protection/disable
Body: {}  // Requires valid session
Response: { "success": true, "status": "disabled" }
```

### Session Management

```
POST /api/auth/login
Body: { "password": "..." }
Response: { "success": true, "session_token": "...", "expires_at": "ISO8601" }

POST /api/auth/logout
Body: { "session_token": "..." }
Response: { "success": true }

GET  /api/auth/validate?token=...
Response: { "valid": true, "expires_at": "ISO8601" }
```

## Migration Path

### Phase 1: Database Schema
1. Add `app_state`, `sessions`, `state_changes` tables
2. Migrate existing config key-value pairs
3. Add database methods for state CRUD

### Phase 2: Session Storage
1. Move `AuthManager.sessions` to database
2. Update login/logout to use database
3. Add session cleanup background task

### Phase 3: Protection State
1. Implement `StateManager` in aegis-core
2. Implement `StateCache` in aegis-proxy
3. Update proxy to use `StateCache.is_filtering_enabled()`

### Phase 4: Dashboard Integration
1. Remove local `ProtectionManager` from UI AppState
2. Use `StateManager` for all state operations
3. Add polling for state change notifications

### Phase 5: Cleanup
1. Remove `FilteringState.enabled` (replaced by StateCache)
2. Remove subprocess-specific workarounds
3. Update tests

## Acceptance Criteria

- [x] Database schema created with `app_state`, `sessions`, `state_changes` tables
- [x] `StateManager` implemented with read/write operations
- [x] `StateCache` implemented with <1ms `is_filtering_enabled()` check
- [x] Session tokens stored in database, validated across processes
- [x] Dashboard pause/resume updates database and proxy sees change within poll interval
- [x] Proxy continues filtering during dashboard restart
- [x] Protection status persists across application restarts
- [x] Pause expiry automatically resumes protection
- [x] All existing tests pass
- [x] Unit tests for StateManager and StateCache

## Performance Requirements

| Metric | Requirement |
|--------|-------------|
| `is_filtering_enabled()` latency | < 100ns (atomic read) |
| State change propagation | < 200ms (polling interval) |
| Database write latency | < 10ms |
| Cache refresh overhead | < 1% CPU |
| Memory overhead | < 1MB for cache |

## Security Considerations

1. **Session tokens** stored with expiry, cleaned up automatically
2. **Pause without auth** - Resume is always allowed (fail-safe)
3. **Disable requires auth** - Prevents unauthorized complete disable
4. **Database file permissions** - Restrict to application user only
5. **No sensitive data in state_changes** - Only key names, not values

## Testing Strategy

### Unit Tests
- StateManager CRUD operations
- StateCache atomic operations
- Session validation with expiry
- Pause expiry handling

### Integration Tests
- Cross-process state synchronization
- Dashboard ↔ Proxy state propagation
- Session validation across processes
- Concurrent state modifications

### Performance Tests
- Cache refresh under load
- State change propagation latency
- Database write throughput

## Notes

### Why Not Shared Memory?

Shared memory (mmap) was considered but rejected because:
1. Platform-specific implementation (Windows vs Unix)
2. Requires careful synchronization primitives
3. No persistence across restarts
4. SQLite already provides ACID guarantees

### Why Polling Instead of Push?

Cross-process push notification is complex:
1. Named pipes require platform-specific code
2. Unix sockets don't exist on Windows
3. TCP localhost adds latency and complexity
4. Polling at 100ms is simple and sufficient

### Future Enhancements

1. **WebSocket for UI** - Real-time updates to dashboard without polling
2. **Event sourcing** - Full audit trail of all state changes
3. **Multi-device sync** - Cloud-backed state for family accounts

//! High-performance state cache for the proxy (F032).
//!
//! This module provides a fast read cache on top of StateManager that minimizes
//! database access during request handling while still responding to state changes.
//!
//! ## Architecture
//!
//! ```text
//! Proxy Request → StateCache.is_filtering_enabled() → Cached Value
//!                        ↓ (async poll)
//!               StateManager → Database
//! ```
//!
//! The cache polls the database at configurable intervals and updates its local
//! state. This ensures that state changes (from dashboard, API, etc.) are reflected
//! in the proxy within the poll interval.

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;

use aegis_storage::{Database, StateManager};

/// Default poll interval for state changes.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// High-performance state cache.
///
/// Provides sub-microsecond reads by caching state locally and polling for
/// changes in the background.
#[derive(Clone)]
pub struct StateCache {
    /// Cached filtering enabled state.
    filtering_enabled: Arc<AtomicBool>,
    /// Last known sequence number for change detection.
    last_seq: Arc<AtomicI64>,
    /// Last poll timestamp.
    last_poll: Arc<RwLock<Instant>>,
    /// Poll interval.
    poll_interval: Duration,
    /// State manager for database access.
    state_manager: StateManager,
}

impl std::fmt::Debug for StateCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateCache")
            .field("filtering_enabled", &self.filtering_enabled.load(Ordering::Relaxed))
            .field("last_seq", &self.last_seq.load(Ordering::Relaxed))
            .field("poll_interval", &self.poll_interval)
            .finish()
    }
}

impl StateCache {
    /// Creates a new state cache with the given database.
    pub fn new(db: Arc<Database>) -> Self {
        let state_manager = StateManager::new(db, "proxy");

        // Initialize cached state from database
        let filtering_enabled = state_manager
            .is_filtering_enabled()
            .unwrap_or(true); // Default to enabled on error

        let last_seq = state_manager.current_seq();

        Self {
            filtering_enabled: Arc::new(AtomicBool::new(filtering_enabled)),
            last_seq: Arc::new(AtomicI64::new(last_seq)),
            last_poll: Arc::new(RwLock::new(Instant::now())),
            poll_interval: DEFAULT_POLL_INTERVAL,
            state_manager,
        }
    }

    /// Creates a state cache with a custom poll interval.
    pub fn with_poll_interval(db: Arc<Database>, poll_interval: Duration) -> Self {
        let mut cache = Self::new(db);
        cache.poll_interval = poll_interval;
        cache
    }

    /// Returns whether filtering is enabled (fast cached read).
    ///
    /// This is the hot path for request handling - just an atomic load.
    #[inline]
    pub fn is_filtering_enabled(&self) -> bool {
        self.filtering_enabled.load(Ordering::Relaxed)
    }

    /// Polls for state changes and updates the cache if needed.
    ///
    /// Call this periodically (e.g., from a background task or at the start
    /// of each request batch). Returns true if state was updated.
    pub fn poll(&self) -> bool {
        // Check if it's time to poll
        let now = Instant::now();
        {
            let last = *self.last_poll.read();
            if now.duration_since(last) < self.poll_interval {
                return false;
            }
        }

        // Update last poll time
        *self.last_poll.write() = now;

        // Check for changes via sequence number
        match self.state_manager.has_changes() {
            Ok(has_changes) => {
                if has_changes {
                    // Refresh cached state from database
                    self.refresh();
                    true
                } else {
                    false
                }
            }
            Err(e) => {
                tracing::warn!("Failed to poll for state changes: {}", e);
                false
            }
        }
    }

    /// Forces a refresh of the cached state from the database.
    pub fn refresh(&self) {
        match self.state_manager.is_filtering_enabled() {
            Ok(enabled) => {
                let old = self.filtering_enabled.swap(enabled, Ordering::SeqCst);
                if old != enabled {
                    tracing::info!(
                        "State cache updated: filtering_enabled {} -> {}",
                        old,
                        enabled
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Failed to refresh state cache: {}", e);
            }
        }

        // Update sequence number
        self.last_seq.store(self.state_manager.current_seq(), Ordering::Relaxed);
    }

    /// Returns the underlying state manager.
    pub fn state_manager(&self) -> &StateManager {
        &self.state_manager
    }

    /// Creates a background polling task.
    ///
    /// Returns a future that should be spawned as a background task.
    /// The task will poll for state changes at the configured interval.
    pub fn start_polling(self: Arc<Self>) -> impl std::future::Future<Output = ()> + Send {
        let cache = self;
        async move {
            loop {
                tokio::time::sleep(cache.poll_interval).await;
                cache.poll();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_cache() -> StateCache {
        let db = Database::in_memory().unwrap();
        StateCache::new(Arc::new(db))
    }

    #[test]
    fn test_default_enabled() {
        let cache = create_test_cache();
        assert!(cache.is_filtering_enabled());
    }

    #[test]
    fn test_poll_updates_state() {
        let db = Database::in_memory().unwrap();
        let db_arc = Arc::new(db);

        // Create cache (starts with enabled=true)
        let cache = StateCache::with_poll_interval(
            db_arc.clone(),
            Duration::from_millis(1), // Very short for testing
        );
        assert!(cache.is_filtering_enabled());

        // Pause protection via state manager
        let manager = StateManager::new(db_arc, "test");
        manager.pause_protection(aegis_storage::PauseDuration::FIFTEEN_MINUTES).unwrap();

        // Poll should detect the change
        std::thread::sleep(Duration::from_millis(5)); // Wait for poll interval
        cache.poll();

        // Should now be disabled
        assert!(!cache.is_filtering_enabled());
    }

    #[test]
    fn test_refresh_updates_state() {
        let db = Database::in_memory().unwrap();
        let db_arc = Arc::new(db);

        let cache = StateCache::new(db_arc.clone());
        assert!(cache.is_filtering_enabled());

        // Pause protection directly via database
        let manager = StateManager::new(db_arc, "test");
        manager.pause_protection(aegis_storage::PauseDuration::FIFTEEN_MINUTES).unwrap();

        // Force refresh
        cache.refresh();

        // Should now be disabled
        assert!(!cache.is_filtering_enabled());
    }

    #[test]
    fn test_debug_impl() {
        let cache = create_test_cache();
        let debug = format!("{:?}", cache);
        assert!(debug.contains("StateCache"));
        assert!(debug.contains("filtering_enabled"));
    }
}

# Changelog

All notable changes to Aegis.

## [Unreleased]

### Added
- Project documentation and specifications
- F001: Project Foundation - Cargo workspace with 6 crates (aegis-core, aegis-storage, aegis-server, aegis-ui, aegis-tray, aegis-app)
- F002: Keyword Classifier - Fast regex-based classifier (<1ms) with patterns for Violence, SelfHarm, Adult, Jailbreak, Hate, Illegal categories
- F003: Prompt Guard ML - ONNX-based ML classifier using Meta's Prompt Guard model for jailbreak/injection detection (<50ms), optional `ml` feature, graceful fallback when model missing
- F004: Tiered Classification - Pipeline orchestrating keyword and ML classifiers with short-circuit on high-confidence matches (<25ms), SafetyClassifier trait, ClassificationTier tracking, graceful degradation when ML unavailable
- F008: SQLite Storage - Privacy-preserving database with events (hash+preview), rules (JSON), config, auth, and daily stats aggregation
- F005: Time Rules - Time-based blocking with day-of-week selection, overnight ranges, enable/disable, and default presets (school night/weekend bedtimes)
- F006: Content Rules - Category-to-action mapping (block/warn/allow) with configurable thresholds and family-safe defaults
- F007: Rule Engine - Unified rule evaluation orchestrating time rules (F005) and content rules (F006), time-first priority, RuleAction/RuleSource tracking, full serialization support
- F013: Authentication - Parent password protection with Argon2 hashing, session management (15min timeout), min 6 char validation, AuthManager/SessionToken/SessionManager types
- F019: User Profiles - Per-child profiles with OS username auto-detection, time/content rules per profile, ProfileManager for lookup, ProfileRepo for persistence, child-safe presets
- F009: HTTP API - Local server (127.0.0.1:8765) with CORS, endpoints: POST /api/check (<100ms), GET /api/stats, GET /api/logs, GET/PUT /api/rules (auth for PUT), POST /api/auth/verify
- F010: Browser Extension - Chrome MV3 extension with content script interception on ChatGPT/Claude/Gemini, checking overlay, block/warn/allow states, service status popup, site-specific handlers
- F011: System Tray - Cross-platform system tray with status indicators (protected/paused/error), menu (Dashboard, Settings, Logs, Pause/Resume, Quit), dynamic shield icon generation, event polling for menu actions
- F012: Parent Dashboard - Native egui desktop GUI with password protection (Argon2), session timeout (15min), sidebar navigation (Dashboard/Profiles/Logs/Settings), summary statistics cards, profile CRUD with editor dialog, time/content rules tabs, activity logs with search/filter/export (CSV), settings with mode selection and password change
- F018: Protection Toggle - Auth-guarded pause/disable with ProtectionState (Active/Paused/Disabled), PauseDuration presets (5min/15min/30min/1hr/indefinite), auto-resume on timed pause expiry, ProtectionManager with thread-safe state, storage persistence via config key-value store
- F014: Notifications - Cross-platform desktop notifications (notify-rust) for blocked content, rate-limited (1/min), shows site and category, enable/disable setting, optional feature flag
- F016: MITM Proxy - Transparent HTTPS proxy (hudsucker) for intercepting LLM traffic across all apps, root CA generation on first run, per-domain certificates on-the-fly, LLM domain filtering (OpenAI/Anthropic/Google), prompt extraction from request bodies, classification and rule engine integration (F007), block page injection, callbacks for block/allow events, graceful shutdown support

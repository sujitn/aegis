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

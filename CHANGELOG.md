# Changelog

All notable changes to Aegis.

## [Unreleased]

### Added
- Project documentation and specifications
- F001: Project Foundation - Cargo workspace with 6 crates (aegis-core, aegis-storage, aegis-server, aegis-ui, aegis-tray, aegis-app)
- F002: Keyword Classifier - Fast regex-based classifier (<1ms) with patterns for Violence, SelfHarm, Adult, Jailbreak, Hate, Illegal categories
- F003: Prompt Guard ML - ONNX-based ML classifier using Meta's Prompt Guard model for jailbreak/injection detection (<50ms), optional `ml` feature, graceful fallback when model missing
- F008: SQLite Storage - Privacy-preserving database with events (hash+preview), rules (JSON), config, auth, and daily stats aggregation
- F005: Time Rules - Time-based blocking with day-of-week selection, overnight ranges, enable/disable, and default presets (school night/weekend bedtimes)
- F006: Content Rules - Category-to-action mapping (block/warn/allow) with configurable thresholds and family-safe defaults

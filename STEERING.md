# Aegis - Steering Document

> Architecture context. Implement from feature specs.

## Vision

Privacy-first AI safety platform that filters LLM interactions.

| Segment | Version |
|---------|---------|
| Aegis Family (MVP) | v1.0 |
| Aegis Team | v2.0 |
| Aegis Enterprise | v3.0 |

## Architecture

```
Browser Extension → Local Rust Service → SQLite
                          │
                   ┌──────┴──────┐
                   ▼             ▼
               Classify       Evaluate
               (Tiered)       (Rules)
```

## Crates

| Crate | Responsibility |
|-------|----------------|
| aegis-core | Classification, rules, auth |
| aegis-storage | SQLite persistence |
| aegis-server | HTTP API (axum) |
| aegis-ui | Settings GUI (egui) |
| aegis-tray | System tray |
| aegis-app | Main binary |

## Key Decisions

- **Browser extension** - Works with enterprise security (no MITM)
- **Local only** - No cloud, privacy first
- **Tiered classification** - Keywords (<1ms) then ML (<50ms)
- **Hash storage** - Store prompt hashes, not content

## Tech Stack

| Component | Technology |
|-----------|------------|
| Language | Rust |
| Async | tokio |
| HTTP | axum |
| ML | ONNX Runtime |
| Database | SQLite |
| UI | egui |
| Extension | TypeScript |

## Performance Targets

| Operation | Target |
|-----------|--------|
| Keyword check | <1ms |
| ML classification | <50ms |
| API response | <100ms |

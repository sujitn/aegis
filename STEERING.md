# Aegis - Steering Document

> Architecture and user experience overview.

## Vision

Privacy-first AI safety for families. Simple install, easy management, automatic protection.

## User Journey

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│  1. DOWNLOAD        2. INSTALL         3. SETUP                     │
│  ────────────       ─────────          ───────                      │
│  aegis.app     →    Run .dmg/.msi  →   Password + Mode + Profile    │
│                                                                      │
│  4. PROTECT         5. MANAGE          6. UPDATE                    │
│  ─────────          ────────           ────────                     │
│  Runs silently  →   Tray/Dashboard →   Auto-notify                  │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        INTERCEPTION LAYER                            │
│                                                                      │
│     Browser Extension          OR           MITM Proxy              │
│     (browser only)                          (all apps)              │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          CORE ENGINE                                 │
│                                                                      │
│   OS User → Profile Lookup → Classify (Keywords/ML) → Apply Rules   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        MANAGEMENT LAYER                              │
│                                                                      │
│   System Tray ◄────► Parent Dashboard ◄────► SQLite Storage         │
│   (quick view)       (full control)          (profiles, rules, logs)│
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Crates

| Crate | Responsibility |
|-------|----------------|
| aegis-core | Classification, rules, profiles, auth |
| aegis-proxy | MITM proxy, TLS, interception |
| aegis-server | HTTP API for extension and dashboard |
| aegis-storage | SQLite persistence |
| aegis-ui | Parent dashboard (egui) |
| aegis-tray | System tray icon and menu |
| aegis-app | Main binary, orchestration |

## Key Concepts

### Interception Modes
| Mode | Coverage | Setup | Best For |
|------|----------|-------|----------|
| Browser Extension | Browser only | Install extension | Simple setup, corporate devices |
| MITM Proxy | All apps | Install CA cert | Full protection, home devices |

### User Profiles
- Each child = one profile
- Profile has: name, OS username, time rules, content rules
- Auto-detect OS user → load correct profile
- Parent profile = unrestricted (or no profile)

### Protection States
| State | Icon | Meaning |
|-------|------|---------|
| Active | 🟢 | Filtering enabled |
| Paused | 🟡 | Temporarily off (resumes automatically) |
| Disabled | 🔴 | Off until re-enabled |

## Tech Stack

| Component | Technology |
|-----------|------------|
| Language | Rust |
| Async | tokio |
| HTTP | axum, hyper |
| Proxy | hudsucker |
| TLS | rcgen, rustls |
| Database | SQLite (rusqlite) |
| UI | Dioxus |
| Tray | tray-item |
| Extension | TypeScript |
| Installer | cargo-bundle, WiX, cargo-deb |
| CI/CD | GitHub Actions |

## Distribution

| Platform | Installer | Extension |
|----------|-----------|-----------|
| macOS | .dmg (signed, notarized) | Chrome Web Store |
| Windows | .msi (signed) | Chrome Web Store |
| Linux | .deb, .rpm, .AppImage | Chrome Web Store |

## Performance Targets

| Operation | Target |
|-----------|--------|
| Keyword check | <1ms |
| ML classification | <50ms |
| Total interception | <100ms |
| Profile switch | <10ms |
| Dashboard open | <500ms |

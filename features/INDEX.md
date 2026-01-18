# Feature Index

## Core Features

| ID | Feature | Status | Priority | Crate | Requires |
|----|---------|--------|----------|-------|----------|
| [F001](F001-project-foundation.md) | Project Foundation | `complete` | critical | workspace | - |
| [F002](F002-keyword-classifier.md) | Keyword Classifier | `complete` | critical | core | F001 |
| [F003](F003-prompt-guard.md) | Prompt Guard ML | `complete` | medium | core | F001 |
| [F004](F004-tiered-classification.md) | Tiered Classification | `complete` | critical | core | F002, F003 |
| [F005](F005-time-rules.md) | Time Rules | `complete` | high | core | F001 |
| [F006](F006-content-rules.md) | Content Rules | `complete` | high | core | F002 |
| [F007](F007-rule-engine.md) | Rule Engine | `complete` | critical | core | F004-F006 |
| [F008](F008-sqlite-storage.md) | SQLite Storage | `complete` | critical | storage | F001 |
| [F025](F025-community-rules.md) | Community Rules | `complete` | high | core | F002, F006 |
| [F031](F031-sentiment-analysis.md) | Sentiment Analysis | `complete` | medium | core | F004, F008, F012 |

## Interception

| ID | Feature | Status | Priority | Crate | Requires |
|----|---------|--------|----------|-------|----------|
| [F009](F009-http-api.md) | HTTP API | `complete` | high | server | F007, F008 |
| [F010](F010-browser-extension.md) | Browser Extension | `complete` | high | extension | F009 |
| [F016](F016-mitm-proxy.md) | MITM Proxy | `complete` | high | proxy | F007 |
| [F017](F017-interception-mode.md) | Interception Mode | `complete` | high | core | F010, F016 |
| [F026](F026-smart-content-parsing.md) | Smart Content Parsing | `complete` | high | proxy | F016 |
| [F027](F027-dynamic-site-registry.md) | Dynamic Site Registry | `complete` | high | core | F008, F016 |

## User Management

| ID | Feature | Status | Priority | Crate | Requires |
|----|---------|--------|----------|-------|----------|
| [F013](F013-authentication.md) | Authentication | `complete` | critical | core | F008 |
| [F018](F018-protection-toggle.md) | Protection Toggle | `complete` | high | core | F011, F013 |
| [F019](F019-user-profiles.md) | User Profiles | `complete` | critical | core | F005, F006, F008 |
| [F029](F029-profile-aware-proxy.md) | Profile-Aware Proxy | `complete` | high | core | F016, F018, F019 |
| [F032](F032-centralized-state.md) | Centralized State | `complete` | critical | storage | F008, F013, F018 |

## UI & Experience

| ID | Feature | Status | Priority | Crate | Requires |
|----|---------|--------|----------|-------|----------|
| [F011](F011-system-tray.md) | System Tray | `complete` | high | tray | F001 |
| [F012](F012-parent-dashboard.md) | Parent Dashboard | `complete` | high | ui | F008, F011, F013 |
| [F014](F014-notifications.md) | Notifications | `complete` | medium | core | F007 |
| [F015](F015-first-run-setup.md) | First-Run Setup | `complete` | high | ui | F012, F013, F017, F019 |
| [F020](F020-clean-uninstall.md) | Clean Uninstall | `complete` | high | app | F013, F016 |
| [F028](F028-background-service.md) | Background Service | `complete` | high | app | F011, F012 |
| [F030](F030-autostart-persistence.md) | Autostart & Persistence | `complete` | high | app | F013, F020, F028 |

## Build & Distribution

| ID | Feature | Status | Priority | Crate | Requires |
|----|---------|--------|----------|-------|----------|
| [F021](F021-build-pipeline.md) | Build Pipeline | `complete` | high | infrastructure | F001 |
| [F022](F022-native-installers.md) | Native Installers | `complete` | high | infrastructure | F021 |
| [F023](F023-auto-update.md) | Auto Update | `complete` | medium | app | F021, F022 |
| [F024](F024-extension-deployment.md) | Extension Deployment | `complete` | high | extension | F010 |

## Implementation Order

### Phase 1: Core 
```
F001 → F002 → F008
```

### Phase 2: Rules & Engine
```
F005 → F006 → F004 → F007
```

### Phase 3: Auth & Profiles
```
F013 → F019
```

### Phase 4: Interception
```
F009 → F010 (Extension)
F016 (Proxy)
F017 (Mode Switch)
```

### Phase 5: UI
```
F011 → F018 → F012 → F015 → F014
```

### Phase 6: Distribution
```
F021 → F022 → F023 → F020
F024 (Chrome Web Store)
```

## User Journey

```
1. INSTALL
   Download → Run Installer → First-Run Wizard
   
2. SETUP (F015)
   Password → Mode (Extension/Proxy) → Create Profile → Done
   
3. DAILY USE
   App runs in background → Tray icon shows status
   Child uses AI → Prompts filtered per profile rules
   
4. MANAGEMENT
   Parent clicks tray → Opens Dashboard → View logs, edit rules
   
5. UPDATE
   Notification → Download → Install → Preserved settings
```

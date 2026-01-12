# Feature Index

| ID | Feature | Status | Priority | Crate | Requires |
|----|---------|--------|----------|-------|----------|
| [F001](F001-project-foundation.md) | Project Foundation | `complete` | critical | workspace | - |
| [F002](F002-keyword-classifier.md) | Keyword Classifier | `complete` | critical | core | F001 |
| [F003](F003-prompt-guard.md) | Prompt Guard ML | `ready` | high | core | F001 |
| [F004](F004-tiered-classification.md) | Tiered Classification | `ready` | critical | core | F002, F003 |
| [F005](F005-time-rules.md) | Time Rules | `ready` | high | core | F001 |
| [F006](F006-content-rules.md) | Content Rules | `ready` | high | core | F002 |
| [F007](F007-rule-engine.md) | Rule Engine | `ready` | critical | core | F004-F006 |
| [F008](F008-sqlite-storage.md) | SQLite Storage | `complete` | critical | storage | F001 |
| [F009](F009-http-api.md) | HTTP API | `ready` | critical | server | F007, F008 |
| [F010](F010-browser-extension.md) | Browser Extension | `ready` | critical | extension | F009 |
| [F011](F011-system-tray.md) | System Tray | `ready` | high | tray | F001 |
| [F012](F012-settings-ui.md) | Settings UI | `ready` | high | ui | F008, F011, F013 |
| [F013](F013-authentication.md) | Authentication | `ready` | critical | core | F008 |
| [F014](F014-notifications.md) | Notifications | `ready` | medium | core | F007 |
| [F015](F015-first-run-setup.md) | First-Run Setup | `ready` | high | ui | F012, F013 |

## Implementation Order

```
F001 → F002 → F008 → F005 → F006 → F004 → F007 → F013 → F009 → F010
                                                    ↓
                                         F011 → F012 → F015 → F014

F003 can be deferred (system works without ML)
```

# F006: Content Rules

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | high | aegis-core |

## Description

Map safety categories to actions with confidence thresholds.

## Dependencies

- **Requires**: F002
- **Blocks**: F007

## Acceptance Criteria

- [x] Map categories to actions (block/warn/allow)
- [x] Configurable thresholds
- [x] Enable/disable per rule
- [x] Default family-safe presets

## Notes

Defaults: Violence 0.7, SelfHarm 0.5, Adult 0.7, Jailbreak 0.8

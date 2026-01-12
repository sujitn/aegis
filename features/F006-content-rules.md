# F006: Content Rules

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | high | aegis-core |

## Description

Map safety categories to actions with confidence thresholds.

## Dependencies

- **Requires**: F002
- **Blocks**: F007

## Acceptance Criteria

- [ ] Map categories to actions (block/warn/allow)
- [ ] Configurable thresholds
- [ ] Enable/disable per rule
- [ ] Default family-safe presets

## Notes

Defaults: Violence 0.7, SelfHarm 0.5, Adult 0.7, Jailbreak 0.8

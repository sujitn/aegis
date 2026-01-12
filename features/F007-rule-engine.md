# F007: Rule Engine

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | critical | aegis-core |

## Description

Evaluate classifications against rules. Combine time and content rules.

## Dependencies

- **Requires**: F004, F005, F006
- **Blocks**: F009

## Acceptance Criteria

- [ ] Time rules checked first
- [ ] Content rules against classification
- [ ] Return first matching action
- [ ] Default allow if no match
- [ ] Track which rule triggered
- [ ] Serializable for storage

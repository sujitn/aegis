# F007: Rule Engine

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | critical | aegis-core |

## Description

Evaluate classifications against rules. Combine time and content rules.

## Dependencies

- **Requires**: F004, F005, F006
- **Blocks**: F009

## Acceptance Criteria

- [x] Time rules checked first
- [x] Content rules against classification
- [x] Return first matching action
- [x] Default allow if no match
- [x] Track which rule triggered
- [x] Serializable for storage

## Implementation

- `RuleEngine` orchestrates time rules (F005) and content rules (F006)
- `RuleAction` enum: Allow, Warn, Block
- `RuleSource` tracks which rule triggered: None, TimeRule, ContentRule
- `RuleEngineResult` combines action and source for full traceability
- All types implement Serialize/Deserialize for storage

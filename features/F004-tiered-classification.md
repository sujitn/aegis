# F004: Tiered Classification

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | critical | aegis-core |

## Description

Orchestrate classifiers: keywords first, ML if needed. Short-circuit on match.

## Dependencies

- **Requires**: F002, F003
- **Blocks**: F007

## Acceptance Criteria

- [x] Keywords checked first
- [x] Short-circuit on high-confidence match
- [x] Fall back to ML if no keyword match
- [x] Works without ML model
- [x] Track which tier produced result
- [x] <25ms typical latency

## Notes

Implement SafetyClassifier trait for extensibility.

# F004: Tiered Classification

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | critical | aegis-core |

## Description

Orchestrate classifiers: keywords first, ML if needed. Short-circuit on match.

## Dependencies

- **Requires**: F002, F003
- **Blocks**: F007

## Acceptance Criteria

- [ ] Keywords checked first
- [ ] Short-circuit on high-confidence match
- [ ] Fall back to ML if no keyword match
- [ ] Works without ML model
- [ ] Track which tier produced result
- [ ] <25ms typical latency

## Notes

Implement SafetyClassifier trait for extensibility.

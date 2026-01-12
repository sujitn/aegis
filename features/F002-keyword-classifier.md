# F002: Keyword Classifier

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | critical | aegis-core |

## Description

Fast regex-based classifier for obvious safety violations. Tier 1 (<1ms).

## Dependencies

- **Requires**: F001
- **Blocks**: F004

## Acceptance Criteria

- [x] Patterns for: violence, self-harm, adult, jailbreak
- [x] Case-insensitive matching
- [x] Return matched categories with confidence
- [x] <1ms classification
- [x] No false positives on safe phrases
- [x] Comprehensive tests

## Notes

Categories: Violence, SelfHarm, Adult, Jailbreak, Hate, Illegal

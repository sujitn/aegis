# F002: Keyword Classifier

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | critical | aegis-core |

## Description

Fast regex-based classifier for obvious safety violations. Tier 1 (<1ms).

## Dependencies

- **Requires**: F001
- **Blocks**: F004

## Acceptance Criteria

- [ ] Patterns for: violence, self-harm, adult, jailbreak
- [ ] Case-insensitive matching
- [ ] Return matched categories with confidence
- [ ] <1ms classification
- [ ] No false positives on safe phrases
- [ ] Comprehensive tests

## Notes

Categories: Violence, SelfHarm, Adult, Jailbreak, Hate, Illegal

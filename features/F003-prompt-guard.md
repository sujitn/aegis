# F003: Prompt Guard ML Classifier

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | high | aegis-core |

## Description

ML classifier using Prompt Guard via ONNX. Tier 2 (<50ms).

## Dependencies

- **Requires**: F001
- **Blocks**: F004

## Acceptance Criteria

- [ ] Load ONNX model from path
- [ ] Tokenize input
- [ ] Return safe/unsafe probabilities
- [ ] <50ms on CPU
- [ ] Graceful fallback if model missing
- [ ] Works in keywords-only mode

## Notes

Model: Meta Prompt Guard. Can be deferred - system works without it.

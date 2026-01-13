# F003: Prompt Guard ML Classifier

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | high | aegis-core |

## Description

ML classifier using Prompt Guard via ONNX. Tier 2 (<50ms).

## Dependencies

- **Requires**: F001
- **Blocks**: F004

## Acceptance Criteria

- [x] Load ONNX model from path
- [x] Tokenize input
- [x] Return safe/unsafe probabilities
- [x] <50ms on CPU
- [x] Graceful fallback if model missing
- [x] Works in keywords-only mode

## Notes

Model: Meta Prompt Guard. Can be deferred - system works without it.

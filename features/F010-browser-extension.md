# F010: Browser Extension

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | critical | browser-extension |

## Description

Chrome extension (MV3) intercepts prompts before sending.

## Dependencies

- **Requires**: F009
- **Blocks**: None

## Acceptance Criteria

- [ ] Intercept on ChatGPT, Claude, Gemini
- [ ] Send to localhost:8765/api/check
- [ ] Show checking overlay
- [ ] Block if unsafe
- [ ] Allow if safe
- [ ] Handle service unavailable
- [ ] Popup shows status

## Notes

Sites: chat.openai.com, chatgpt.com, claude.ai, gemini.google.com

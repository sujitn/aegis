# F010: Browser Extension

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | critical | browser-extension |

## Description

Chrome extension (MV3) intercepts prompts before sending.

## Dependencies

- **Requires**: F009
- **Blocks**: None

## Acceptance Criteria

- [x] Intercept on ChatGPT, Claude, Gemini
- [x] Send to localhost:8765/api/check
- [x] Show checking overlay
- [x] Block if unsafe
- [x] Allow if safe
- [x] Handle service unavailable
- [x] Popup shows status

## Implementation

- `extension/` - Chrome MV3 extension (TypeScript)
- `manifest.json` - Extension manifest with host permissions
- `src/api.ts` - API client for Aegis server
- `src/content.ts` - Content script for prompt interception
- `src/background.ts` - Service worker for status monitoring
- `src/popup.ts` - Popup UI script
- `src/sites/` - Site-specific handlers (ChatGPT, Claude, Gemini)
- `overlay.css` - Checking/blocked/warning overlay styles
- `popup.html/css` - Extension popup UI

### Build

```bash
cd extension
npm install
npm run build
```

Then load `extension/` as unpacked extension in Chrome.

## Notes

Sites: chat.openai.com, chatgpt.com, claude.ai, gemini.google.com

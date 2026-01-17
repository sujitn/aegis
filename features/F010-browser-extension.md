# F010: Browser Extension

| Status | Priority | Crate |
|--------|----------|-------|
| `in-progress` | critical | browser-extension |

## Description

Chrome/Edge extension (MV3) intercepts prompts before sending to AI chatbots. Uses a **hybrid interception approach** for maximum reliability:

1. **Primary: Network Request Interception** - Intercepts fetch/XHR calls to LLM API endpoints. Immune to DOM/UI changes.
2. **Fallback: DOM-based Interception** - Site-specific handlers as backup for edge cases.

## Dependencies

- **Requires**: F009 (Local API Server)
- **Blocks**: None

## Acceptance Criteria

### Core Functionality
- [x] Intercept on ChatGPT, Claude, Gemini
- [x] Send to localhost:8765/api/check
- [x] Show checking overlay
- [x] Block if unsafe
- [x] Allow if safe
- [x] Handle service unavailable (fail-open/fail-closed modes)
- [x] Popup shows status

### Network Interception (New)
- [x] Intercept fetch() requests to LLM API endpoints
- [x] Intercept XMLHttpRequest to LLM API endpoints
- [x] Extract prompts from JSON request bodies
- [x] Support multiple payload formats (OpenAI, Anthropic, Google)
- [x] Handle streaming responses
- [x] Support additional LLM services (Copilot, Perplexity, Poe)

### WebSocket Interception (New)
- [x] Intercept WebSocket connections to LLM services
- [x] Extract prompts from WebSocket messages (socket.io, SignalR)
- [x] Monitor incoming WebSocket messages for response content
- [x] Support Perplexity (socket.io), Copilot (SignalR), Poe WebSocket protocols

### Easy Installation (F024-alt)
- [ ] Package as CRX for manual install
- [ ] Auto-install via native app (Windows registry, macOS/Linux policies)
- [ ] Provide clear manual installation instructions

## Implementation

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Content Script                            │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐    ┌─────────────────────────────┐ │
│  │ Network Interceptor │    │   DOM Handlers (Fallback)   │ │
│  │  (Primary Method)   │    │   - ChatGPT handler         │ │
│  │                     │    │   - Claude handler          │ │
│  │  - Intercept fetch  │    │   - Gemini handler          │ │
│  │  - Intercept XHR    │    │   - Generic handler         │ │
│  │  - Extract prompts  │    │                             │ │
│  │  - Block/Allow      │    │                             │ │
│  └──────────┬──────────┘    └──────────────┬──────────────┘ │
│             │                              │                 │
│             └──────────────┬───────────────┘                 │
│                            ▼                                 │
│                    ┌───────────────┐                         │
│                    │  Aegis API    │                         │
│                    │  (localhost)  │                         │
│                    └───────────────┘                         │
└─────────────────────────────────────────────────────────────┘
```

### Files

```
extension/
├── manifest.json           # Extension manifest (MV3)
├── src/
│   ├── api.ts              # API client for Aegis server
│   ├── content.ts          # Main content script (orchestrates both methods)
│   ├── interceptor.ts      # Network request interceptor (NEW)
│   ├── background.ts       # Service worker for status monitoring
│   ├── popup.ts            # Popup UI script
│   └── sites/              # Site-specific DOM handlers (fallback)
│       ├── types.ts        # Handler interface
│       ├── index.ts        # Handler registry
│       ├── chatgpt.ts      # ChatGPT DOM handler
│       ├── claude.ts       # Claude DOM handler
│       └── gemini.ts       # Gemini DOM handler
├── overlay.css             # Checking/blocked/warning overlay styles
├── popup.html              # Extension popup UI
└── icons/                  # Extension icons
```

### API Endpoints Intercepted

| Service | API Endpoint Pattern |
|---------|---------------------|
| ChatGPT | `https://chatgpt.com/backend-api/conversation` |
| ChatGPT | `https://chat.openai.com/backend-api/conversation` |
| Claude | `https://claude.ai/api/*/chat_conversations/*/completion` |
| Gemini | `https://gemini.google.com/_/BardChatUi/data/*` |
| Copilot | `https://copilot.microsoft.com/c/api/conversations` |
| Perplexity | `https://www.perplexity.ai/socket.io/*` |
| Poe | `https://poe.com/api/gql_POST` |

### Prompt Extraction

The interceptor extracts prompts from various JSON formats:

```typescript
// ChatGPT format
{ messages: [{ content: { parts: ["prompt"] } }] }
{ messages: [{ content: "prompt" }] }

// Claude format
{ prompt: "text" }
{ message: "text" }

// OpenAI-compatible format
{ messages: [{ role: "user", content: "prompt" }] }
```

### Build

```bash
cd extension
npm install
npm run build
```

### Installation Options

1. **Developer Mode (Current)**
   - Enable Developer Mode in chrome://extensions
   - Click "Load unpacked"
   - Select the `extension/` directory

2. **CRX Package**
   - Build extension
   - Package as .crx file
   - Distribute for manual installation

3. **Native App Auto-Install** (Planned - see F024)
   - Aegis app registers extension via OS policies
   - Extension installs automatically when user opens Chrome

## Notes

### Why Network Interception?

DOM-based interception is fragile because:
- CSS selectors break when sites update their UI
- Class names and test IDs change frequently
- Different A/B test variants have different DOM structures

Network interception is robust because:
- API endpoints rarely change (breaking changes affect all clients)
- Request/response formats are stable (versioned APIs)
- Works regardless of UI framework or styling

### Fail Modes

- **Fail-Closed (Default)**: Block prompts when Aegis service unavailable
- **Fail-Open**: Allow prompts when Aegis service unavailable (less safe)

### Supported Sites

- chat.openai.com / chatgpt.com (ChatGPT) - fetch interception
- claude.ai (Claude) - fetch interception
- gemini.google.com (Gemini) - fetch interception
- copilot.microsoft.com / bing.com/chat (Microsoft Copilot) - fetch + WebSocket (SignalR)
- perplexity.ai (Perplexity) - fetch + WebSocket (socket.io)
- poe.com (Poe) - fetch + WebSocket (GraphQL)

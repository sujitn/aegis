# F016: MITM Proxy

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | critical | aegis-proxy |

## Description

Transparent HTTPS proxy that intercepts LLM traffic. Catches all apps (browser, desktop, CLI).

## Dependencies

- **Requires**: F007 (Rule Engine)
- **Blocks**: None

## Acceptance Criteria

- [x] Generate root CA on first run
- [x] Generate per-domain certificates on the fly
- [x] Intercept only LLM domains (passthrough others)
- [x] Extract prompt from request body
- [x] Apply classification and rules (F007)
- [x] Block or forward based on result
- [x] Inject block page for blocked requests
- [x] Log events to storage (F008)
- [x] <100ms added latency
- [x] Works with browser and desktop apps

## Target Domains

```
api.openai.com
chat.openai.com
chatgpt.com
claude.ai
api.anthropic.com
gemini.google.com
generativelanguage.googleapis.com
```

## Notes

New crate: aegis-proxy. Add to workspace Cargo.toml.

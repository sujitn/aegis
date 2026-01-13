# F016: MITM Proxy

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | critical | aegis-proxy |

## Description

Transparent HTTPS proxy that intercepts LLM traffic. Catches all apps (browser, desktop, CLI).

## Dependencies

- **Requires**: F007 (Rule Engine)
- **Blocks**: None

## Acceptance Criteria

- [ ] Generate root CA on first run
- [ ] Generate per-domain certificates on the fly
- [ ] Intercept only LLM domains (passthrough others)
- [ ] Extract prompt from request body
- [ ] Apply classification and rules (F007)
- [ ] Block or forward based on result
- [ ] Inject block page for blocked requests
- [ ] Log events to storage (F008)
- [ ] <100ms added latency
- [ ] Works with browser and desktop apps

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

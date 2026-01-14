# F026: Smart Content Parsing

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | high | aegis-proxy |

## Description

Robust prompt extraction from diverse LLM payload formats. Current `extractor.rs` handles JSON APIs but lacks support for form data, multipart, streaming, and chat history differentiation. Extensible parser registry for new formats.

## Dependencies

- **Requires**: F016
- **Blocks**: None

## Current State

`crates/aegis-proxy/src/extractor.rs`:
- OpenAI/ChatGPT JSON (messages array, multimodal)
- Anthropic JSON (messages, content blocks)
- Google/Gemini JSON (contents, parts)
- Generic fallback (prompt/text/query fields)
- WebSocket JSON (handler.rs)

## Acceptance Criteria

### Format Support

- [ ] JSON (existing, enhanced)
- [ ] Form data (`application/x-www-form-urlencoded`)
- [ ] Multipart form data (`multipart/form-data`)
- [ ] NDJSON/JSON Lines (streaming batch)
- [ ] Raw text (`text/plain`)

### JSON Parsing Enhancement

- [ ] Detect content-type header, fall back to sniffing
- [ ] Handle BOM and charset variations
- [ ] Graceful handling of truncated/malformed JSON
- [ ] Support JSON5 quirks (trailing commas, comments)

### Chat History vs Current Prompt

- [ ] Identify "current" message (last user message by default)
- [ ] Option to scan full history or current only
- [ ] Weight current prompt higher in classification
- [ ] Track conversation context for multi-turn attacks

### Streaming Requests

- [ ] Server-Sent Events (SSE) `text/event-stream`
- [ ] Chunked transfer encoding
- [ ] Accumulate streaming chunks before classification
- [ ] Configurable buffer size and timeout
- [ ] Early exit on clear violation (don't wait for full stream)

### Fallback Strategy

- [ ] Unknown format: extract all string content > 10 chars
- [ ] Binary detection: skip non-text payloads
- [ ] Size limit: truncate at configurable max (default 1MB)
- [ ] Return extraction confidence score

### Parser Registry

- [ ] Trait-based parser interface:
  ```
  trait PayloadParser: Send + Sync {
      fn can_parse(&self, content_type: &str, host: &str) -> bool;
      fn parse(&self, body: &[u8], context: &ParseContext) -> ParseResult;
  }
  ```
- [ ] Register parsers by priority
- [ ] First matching parser wins
- [ ] Built-in parsers: JSON, Form, Multipart, Text
- [ ] Custom parser registration API

### Performance

- [ ] Lazy parsing: only parse if LLM domain
- [ ] Zero-copy where possible (Bytes, slices)
- [ ] Early content-type rejection (skip images, binaries)
- [ ] Benchmark: < 1ms for typical JSON payload
- [ ] Benchmark: < 5ms for 100KB payload
- [ ] Cache parsed results for retry scenarios

## Notes

Parse context includes:
- Host/domain
- Path
- Content-Type header
- Content-Length
- Request method

ParseResult returns:
- Extracted prompts (Vec<ExtractedPrompt>)
- Current vs history flag per prompt
- Confidence score (0.0-1.0)
- Parser name (for logging)
- Warnings (partial parse, truncated, etc.)

Streaming accumulation:
- Buffer until: complete JSON object, newline delimiter, timeout, or size limit
- For SSE: parse `data:` lines, accumulate until `event: done` or timeout

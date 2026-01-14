# F027: Dynamic Site Registry

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | high | aegis-core |

## Description

Replace hardcoded `LLM_DOMAINS` array with dynamic registry. Parents can add custom endpoints, disable defaults, and receive updates for new services. Links to F026 for parser mapping.

## Dependencies

- **Requires**: F008, F016
- **Blocks**: None

## Current State

`crates/aegis-proxy/src/domains.rs`:
- Hardcoded `LLM_DOMAINS: &[&str]` array (22 domains)
- Simple `ends_with()` subdomain matching
- `service_name()` returns hardcoded friendly names
- No persistence, no customization

## Acceptance Criteria

### Site Entry Schema

- [ ] Domain pattern (exact or wildcard)
- [ ] Friendly name (display in logs/UI)
- [ ] Category: `consumer` | `api` | `enterprise`
- [ ] Parser ID (links to F026 registry)
- [ ] Enabled flag
- [ ] Source: `bundled` | `remote` | `custom`
- [ ] Priority (for overlapping patterns)

### Bundled Default List

- [ ] Embed default sites in binary (compile-time)
- [ ] Include all current `LLM_DOMAINS` entries
- [ ] Categorize existing sites:
  - Consumer: chatgpt.com, claude.ai, gemini.google.com
  - API: api.openai.com, api.anthropic.com, api.x.ai
  - Enterprise: (future)
- [ ] Version hash for update detection

### Wildcard Patterns

- [ ] Support `*.domain.com` (any subdomain)
- [ ] Support `**.domain.com` (any depth subdomain)
- [ ] Exact match takes priority over wildcard
- [ ] More specific patterns win (*.api.openai.com > *.openai.com)

### Parent Customization

- [ ] Add custom domain via dashboard
- [ ] Validate domain format (no scheme, valid chars)
- [ ] Assign parser (default: auto-detect from F026)
- [ ] Set category (default: enterprise)
- [ ] Toggle enabled/disabled per site
- [ ] Disable bundled sites (not delete)
- [ ] Re-enable disabled bundled sites

### Parser Mapping (F026 Integration)

- [ ] Each site entry has optional `parser_id`
- [ ] `parser_id` references F026 parser registry
- [ ] Default: `auto` (F026 selects by content-type/host)
- [ ] Override: force specific parser for domain
- [ ] Mapping stored in site entry, not separate table

### Remote Updates

- [ ] Check for updates on app launch (async)
- [ ] Update endpoint: configurable URL
- [ ] Delta updates (additions/removals/changes)
- [ ] Merge strategy: remote adds new, doesn't override custom
- [ ] Parent can disable remote updates
- [ ] Manual "Check Now" button in dashboard
- [ ] Show "last updated" timestamp

### Persistence

- [ ] Store custom sites in SQLite
- [ ] Store disabled bundled sites in SQLite
- [ ] Cache remote updates in SQLite
- [ ] Export/import custom sites (JSON)

### API

- [ ] `SiteRegistry::is_monitored(host: &str) -> bool`
- [ ] `SiteRegistry::get_site(host: &str) -> Option<&SiteEntry>`
- [ ] `SiteRegistry::service_name(host: &str) -> &str`
- [ ] `SiteRegistry::parser_id(host: &str) -> Option<&str>`
- [ ] `SiteRegistry::add_custom(entry: SiteEntry)`
- [ ] `SiteRegistry::set_enabled(pattern: &str, enabled: bool)`
- [ ] `SiteRegistry::reload()` (refresh from DB + remote)

### Performance

- [ ] O(1) lookup for exact matches (HashMap)
- [ ] Efficient wildcard matching (trie or sorted patterns)
- [ ] Cache resolved lookups (LRU, 1000 entries)
- [ ] Benchmark: < 100μs per lookup

## Notes

Site entry example:
```json
{
  "pattern": "*.openai.com",
  "name": "OpenAI",
  "category": "api",
  "parser_id": "openai_json",
  "enabled": true,
  "source": "bundled",
  "priority": 10
}
```

Categories:
- `consumer`: Web chat interfaces (chatgpt.com, claude.ai)
- `api`: Developer APIs (api.openai.com, api.anthropic.com)
- `enterprise`: Self-hosted, corporate (azure openai, bedrock)

Wildcard resolution order:
1. Exact match: `api.openai.com`
2. Single wildcard: `*.openai.com`
3. Double wildcard: `**.openai.com`
4. Higher priority wins ties

# F025: Community Rules

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | high | aegis-core |

## Description

Integrate open-source safety databases into the keyword classifier. Layer community rules with Aegis-curated patterns and parent customizations. Bundle with app and support updates.

## Dependencies

- **Requires**: F002, F006
- **Blocks**: None

## Sources

| Database | License | Content | Languages |
|----------|---------|---------|-----------|
| Surge AI Profanity | MIT | 1,600+ profanities, 10 categories, severity ratings | EN (20+ planned) |
| LDNOOBW | CC-BY-4.0 | Bad word lists, 31 languages | Multi |
| JailbreakBench | MIT | 200 jailbreak behaviors, OpenAI categories | EN |
| PromptInject | MIT | Goal hijacking, prompt leak patterns | EN |

## Acceptance Criteria

### Integration

- [ ] Load community rules at classifier initialization
- [ ] Convert external formats (CSV, JSON, TXT) to internal regex patterns
- [ ] Map external categories to Aegis categories (Violence, Adult, Jailbreak, etc.)
- [ ] Preserve severity ratings where available

### Rule Layering

- [ ] Three-tier priority: Community (lowest) < Aegis Curated (medium) < Parent Custom (highest)
- [ ] Higher tier rules override lower tier for same pattern
- [ ] Parent can disable any community/curated rule
- [ ] Parent can whitelist terms blocked by lower tiers

### Bundling

- [ ] Bundle compiled rule sets in app binary (embedded resources)
- [ ] Store bundled version hash for update detection
- [ ] Rules load without network access (offline-first)
- [ ] Total bundle size < 5MB

### Updates

- [ ] Check for rule updates on app launch (background, non-blocking)
- [ ] Download delta updates when available
- [ ] Store updated rules in app data directory
- [ ] Rollback mechanism if update fails validation
- [ ] Manual "Check for Updates" in dashboard

### Multi-Language

- [ ] Language detection from system locale
- [ ] Load language-specific rules automatically
- [ ] Fallback to English if locale unavailable
- [ ] Parent can enable additional languages

### Parent Override

- [ ] UI to view all active rules by tier
- [ ] Toggle individual community rules on/off
- [ ] Custom whitelist: terms to never block
- [ ] Custom blacklist: additional terms to block
- [ ] Export/import parent customizations

## Notes

Category mapping:
- Surge AI "Sexual anatomy" / "Sexual acts" -> Adult
- Surge AI "Racial/ethnic slurs" -> Hate
- LDNOOBW words -> map by context or flag as Profanity (new category)
- JailbreakBench behaviors -> Jailbreak
- PromptInject patterns -> Jailbreak

Severity mapping:
- Surge AI Mild -> confidence 0.6
- Surge AI Strong -> confidence 0.8
- Surge AI Severe -> confidence 0.95

# CLAUDE.md

> Read this first in every Claude Code session.

## Project

**Aegis** - AI safety platform for filtering LLM interactions (parental controls MVP).

## Files

- `STEERING.md` - Architecture context
- `features/INDEX.md` - Feature list and status
- `features/F*.md` - Feature specifications
- `agents/AGENTS.md` - Quality checklist

## Workflow

1. **Pick** - Check `features/INDEX.md` for `ready` features
2. **Read** - Read the feature spec `features/F00X-*.md`
3. **Implement** - Update status to `in-progress`, build it, write tests
4. **Validate** - Run `agents/AGENTS.md` checklist
5. **Complete** - Update status to `complete`, update `CHANGELOG.md`

## Commands

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt
```

## Adding Features

1. Copy `features/TEMPLATE.md` → `features/F0XX-name.md`
2. Fill in description and acceptance criteria
3. Add to `features/INDEX.md`
4. Set status to `ready`

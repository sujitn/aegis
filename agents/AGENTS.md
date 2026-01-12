# Quality Agents

> Run before completing any feature.

## SOLID

- [ ] Single Responsibility - Each type does one thing
- [ ] Open/Closed - Extend without modifying
- [ ] Liskov Substitution - Implementations honor contracts
- [ ] Interface Segregation - Traits are minimal
- [ ] Dependency Inversion - Depend on abstractions

## DRY

- [ ] No copy-pasted code
- [ ] Constants defined once
- [ ] Common patterns extracted

## KISS

- [ ] Simplest solution that works
- [ ] No premature optimization
- [ ] Clear over clever

## Testing

- [ ] Happy path tested
- [ ] Error cases tested
- [ ] Edge cases tested
- [ ] Tests are deterministic

## Architecture

- [ ] Correct layer (core → server → ui)
- [ ] Dependencies flow inward
- [ ] No circular dependencies

## Performance

- [ ] Meets latency targets
- [ ] No blocking I/O on async
- [ ] Resources bounded

## Security

- [ ] Passwords hashed (Argon2)
- [ ] SQL parameterized
- [ ] Input validated
- [ ] Errors don't leak info

## When to Apply

| Feature Type | Agents |
|--------------|--------|
| All | SOLID, DRY, KISS, Testing |
| New modules | + Architecture |
| Hot path | + Performance |
| Auth/Storage/API | + Security |

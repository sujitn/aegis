# F005: Time-Based Rules

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | high | aegis-core |

## Description

Block AI access based on time/day. Bedtime and school hours.

## Dependencies

- **Requires**: F001
- **Blocks**: F007

## Acceptance Criteria

- [ ] Day-of-week selection
- [ ] Time ranges (start/end)
- [ ] Overnight ranges (9pm-7am)
- [ ] Multiple rules coexist
- [ ] Enable/disable per rule
- [ ] Default presets

## Notes

Defaults: Bedtime 9pm-7am (school nights), 11pm-8am (weekends). School hours disabled.

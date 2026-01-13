# F005: Time-Based Rules

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | high | aegis-core |

## Description

Block AI access based on time/day. Bedtime and school hours.

## Dependencies

- **Requires**: F001
- **Blocks**: F007

## Acceptance Criteria

- [x] Day-of-week selection
- [x] Time ranges (start/end)
- [x] Overnight ranges (9pm-7am)
- [x] Multiple rules coexist
- [x] Enable/disable per rule
- [x] Default presets

## Notes

Defaults: Bedtime 9pm-7am (school nights), 11pm-8am (weekends). School hours disabled.

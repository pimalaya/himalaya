---
cairn: log
change: a-google-daily-limit-is-rate-limited
landed: 2026-11-18
---

# A Google daily limit is rate-limited

Finding COH-4 of the family's cli-family-analysis-2026-11-15 plan, owner ruling of 2026-11-18.

## What landed

- src/gmail/client.rs `is_daily_limit`, read by `refusal_of` for a whole command and a batch item.
- Test `a_daily_limit_is_rate_limited`, failing on 002bf012.
- CHANGELOG (Changed, breaking), MIGRATION.

## Capabilities moved

- [commands](../spec/commands.md): the `permission-denied` and `rate-limited` rows of the code table.

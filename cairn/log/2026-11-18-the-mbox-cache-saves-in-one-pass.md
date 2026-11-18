---
cairn: log
change: the-mbox-cache-saves-in-one-pass
landed: 2026-11-18
---

# The mbox cache saves in one pass

Finding HIM-16 of the family's cli-family-analysis-2026-11-15 plan, its mbox cache half.

## What landed

- src/mbox/cache.rs: the save in one pass.
- No regression test (complexity alone, no timing run); the existing mbox tests pass.
- CHANGELOG (Fixed). Left: the quadratic id deduplication (src/shared/flag/arg.rs, src/email/outcome.rs) and the Maildir per-id lookups.

## Capabilities moved

- [backends](../spec/backends.md): *mbox reads go through a cache*.

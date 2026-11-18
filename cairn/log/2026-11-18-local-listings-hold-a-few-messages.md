---
cairn: log
change: local-listings-hold-a-few-messages
landed: 2026-11-18
---

# Local listings hold a few messages

Finding HIM-8 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/maildir/backend.rs `visit_entries`; src/mbox/backend.rs `envelopes` with a `keep` closure.
- No regression test (memory alone, not measured); the Maildir, mbox and envelope search suites pass.
- CHANGELOG (Fixed). Left: the mbox cache still saved after the loop only.

## Capabilities moved

- [backends](../spec/backends.md): *A local listing holds a few messages at a time*, new.

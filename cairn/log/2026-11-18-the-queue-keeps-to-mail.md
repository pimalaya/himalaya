---
cairn: log
change: the-queue-keeps-to-mail
landed: 2026-11-18
---

# The queue keeps to mail

Finding TRI-1 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/pimdir/backend.rs: `queue_row` keeps to the account's mail collections.
- Test `a_queue_row_of_another_kind_is_unknown`, failing before the fix.
- CHANGELOG (Fixed).

## Capabilities moved

- [backends](../spec/backends.md): *The pimdir subcommand reads and retracts the queue*.

---
cairn: log
change: the-queue-lists-the-account
landed: 2026-11-18
---

# The queue lists the account

Finding TRI-5 of the family's cli-family-analysis-2026-11-15 plan, ruled by the owner on 2026-11-18.

## What landed

- src/pimdir/queue/list.rs: `pimdir queue list` lists the whole account, `-m/--mailbox` narrowing it; src/pimdir/backend.rs `mail_collection_ids`.
- Test `the_queue_list_covers_the_account_unless_narrowed`, failing on 72c1c0ca.
- CHANGELOG (Changed, breaking), MIGRATION (development build).

## Capabilities moved

- [backends](../spec/backends.md): *The pimdir subcommand reads and retracts the queue*, *The pimdir queue view shows parked rows*, *The queue view shows queued sends*.

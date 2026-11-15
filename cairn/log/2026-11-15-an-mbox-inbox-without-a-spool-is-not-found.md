---
cairn: log
change: an-mbox-inbox-without-a-spool-is-not-found
landed: 2026-11-15
---

# An mbox inbox without a spool is not found

Task H23 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (mbox.md, H-5): a missing mbox spool failed uncoded.

## What landed

- src/mbox/client.rs: `resolve_mbox` refuses `INBOX` without a spool or a root file of that name through `mailbox_not_found`, so it is coded `mailbox-not-found`.
- Test: tests/local_stores.rs `an_mbox_inbox_without_a_spool_is_not_found`, coded `null` before.

## Capabilities moved

- [backends](../spec/backends.md): *mbox backend*, the inbox without a spool coded `mailbox-not-found`.

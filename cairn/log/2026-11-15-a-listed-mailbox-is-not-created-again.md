---
cairn: log
change: a-listed-mailbox-is-not-created-again
landed: 2026-11-15
---

# A listed mailbox is not created again

Task H29 of the family's cli-alignment-2026-11-15 plan, with neverest's N21, from live testing on 2026-11-15 (part2-pimdir.md, F3): a pimdir `mailbox create` of a mailbox the store listed was queued, and the sync reported it done while iCloud refused the `CREATE`.

## What landed

- src/pimdir/backend.rs: `create_mailbox` refuses, coded `item-duplicate` and naming it, a mailbox the performer's source syncs under the name at the top or under the parent (its name, `/` or `.`, then the name), and a creation of it the source has queued, before anything is queued (`held_mailbox`, `creation_queued`). src/error.rs: `ItemDuplicate` covers it.
- Test `a_mailbox_the_store_lists_is_not_created_again`, failing without the checks.

## Capabilities moved

- [backends](../spec/backends.md): *pimdir creates a mailbox by queueing an intent* refuses a listed or queued mailbox, with a scenario.
- [commands](../spec/commands.md): the `item-duplicate` row of the code table.

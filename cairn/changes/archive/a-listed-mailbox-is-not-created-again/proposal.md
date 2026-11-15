---
cairn: change
id: a-listed-mailbox-is-not-created-again
status: landed
created: 2026-11-15
---

# A listed mailbox is not created again

## Why

Task H29 of the family's cli-alignment-2026-11-15 plan, with neverest's N21, from live testing on 2026-11-15 (reports/himalaya/part2-pimdir.md, F3; imap-smtp-icloud.md, P6): `himalaya -a icloud-pimdir mailbox create Pimalaya-Test-himalaya-1-renamed`, while the store listed `icloud/imap/Pimalaya-Test-himalaya-1-renamed`, was queued, and the next sync reported it done while iCloud answers such a `CREATE` with `NO Mailbox already exists`. commands.md "Unified mailbox writes" says creating an existing mailbox SHALL be refused, and the store lists it, so Himalaya can tell before queueing.

## What

`create_mailbox` of the pimdir backend, under the unified `mailbox create` and `pimdir mailbox create`, refuses a mailbox the performer's source already syncs under the name asked, at the top or under the parent, and a creation of it that source has queued already, coded `item-duplicate` (the family's duplicate code; Himalaya had none for an existing mailbox, Maildir's refusal being uncoded), before anything is queued. The store records no hierarchy delimiter, so a child of the parent reads as the parent's name, `/` or `.`, then the name; names compare exactly, so a case variant is left to the server. A name the store does not list is still the server's to refuse, the row parking (N21 makes neverest park it).

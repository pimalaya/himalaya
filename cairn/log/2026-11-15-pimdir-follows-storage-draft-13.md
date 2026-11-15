---
cairn: log
change: pimdir-follows-storage-draft-13
landed: 2026-11-15
---

# Pimdir follows storage-draft-13

Line N31 of the family's cli-alignment-2026-11-15 plan.

## What landed

- io-pimdir 0.14.0 (storage-draft-13, sync-draft-13, search-draft-13), from crates.io, no [patch] section.
- `pimdir queue show` prints the refusals standing on an applied row's message (`refusals`, a `Refused by` line each), `seq` naming the message any applied row acted on; `pimdir queue list` prints the mailbox's refusals under `refused`.
- A store of another draft, a draft-11 one among them, is refused `store-version` with the rebuild hint Cardamum and Calendula print, naming both drafts.

## Capabilities moved

- [backends](../spec/backends.md): *A pimdir store failure carries its code* and *The pimdir subcommand reads and retracts the queue* modified.
- [search](../spec/search.md) and [commands](../spec/commands.md): the draft named and the `store-version` hint.

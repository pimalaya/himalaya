---
cairn: change
id: pimdir-follows-storage-draft-13
status: landed
created: 2026-11-15
---

# Pimdir follows storage-draft-13

## Why

Line N31 of the family's cli-alignment-2026-11-15 plan. io-pimdir 0.14.0 follows pimdir storage-draft-13, sync-draft-13 and search-draft-13: a source's server may refuse a pushed change for good, the refusal standing on the item's binding, read through the receipt (`PimdirActionStatus::Applied::refusals`) and per collection (`PimdirReader::refusals`), and STORAGE §15.4 asks a producer to show such a row applied and refused, with the reason. A store of an earlier draft is refused, never migrated. Cardamum and Calendula print the same in `pimdir queue show` and `pimdir status`; Himalaya has no `pimdir status`, so its `pimdir queue list` carries the mailbox's refusals, the view STORAGE puts beside the queue.

## What

- io-pimdir 0.14.0.
- `pimdir queue show`: `refusals` on an applied row, a `Refused by` line each; `seq` the message the row acted on.
- `pimdir queue list`: `refused`, the mailbox's refusals.
- A store of another draft reads the twins' rebuild hint, naming both drafts.

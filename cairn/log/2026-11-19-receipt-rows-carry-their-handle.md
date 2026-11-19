---
cairn: log
change: receipt-rows-carry-their-handle
landed: 2026-11-19
---

# Receipt rows carry their handle

The owner's ruling of 2026-11-19 on the change, as recommended: a pimdir queue row prints `row`, the integer row id, with `handle` beside it, as Calendula and Cardamum print them and pimalaya-cli's output.md requires of `row`.

## What landed

- src/pimdir/client.rs: `PimdirClient::store_identity` (from `change_cursor`), `named` (a bare id named under the store's identity) and the free `handle`.
- src/pimdir/message/add.rs, send.rs (`copyRow` with `copyHandle`), mailbox/create.rs, rename.rs, delete.rs, flag/mod.rs, index.rs and tag.rs (`null` both when nothing was staged): `row` an integer, `handle` a string.
- src/pimdir/queue/list.rs and backend.rs: each row carries `handle`, printed in place of the bare id in text.
- src/pimdir/queue/show.rs and cancel.rs: `{row, handle, ...}`, a bare id given named under the store's identity, where `row` echoed the string given.
- Tests: the output tests of each receipt, `queue list`, `show` and `cancel` assert the new shape (they fail to build against the old one, its fields gone), and `a_listed_row_carries_the_receipt_row_and_handle` checks that a receipt, the listed row and a `queue show` given the bare id agree.
- The schemas follow the types (`json-schema`).
- CHANGELOG (Changed, breaking, the unreleased string `row` entry rewritten) and MIGRATION (v2 to v3, and the development build section).

## Capabilities moved

- backends: *A queue row is named by its handle*, *The pimdir subcommand reads and retracts the queue*, *pimdir writes name their queue row* (MODIFIED).
- commands: *Text output is plain text*, the row cell of `pimdir tag` and `pimdir queue list` (MODIFIED).

## Left

- pimalaya-cli's output.md names `handle` in the shared queue row (cross-repo).
- Calendula and Cardamum print `{row, kind, unconfirmed}` from `queue cancel` and no `handle` from `queue show`: Himalaya prints `handle` in both, which the twins follow at their own pass.

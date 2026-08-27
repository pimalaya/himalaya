---
cairn: log
change: pimdir-queue-visibility
date: 2026-08-27
---

# A staged write shows on the next read, not on the next sync

`pimdir-producer-reader` made Himalaya a reader and a producer, and left the two halves unconnected: a write went to the store's queue, a read projected the committed index, and nothing joined them. Flag a message and the flag was gone from the listing until Neverest ran. Capabilities `backends` and `commands` moved.

## What landed

- **Reads go through an overlaying `PimdirReader`** (capability `backends`). `PimdirClient` holds the reader role rather than a store handle, so the backend carries no write at all, and builds it with `with_pending()`, so a staged `set-flags`, `update`, `remove`, `move` or `copy` shows on the next read. Every one of those addresses a message that already exists and keeps its public id, so nothing changed about how a message is addressed and `Envelope.id` is still a `String`.

- **A queued creation is reported, not listed.** It has no `seq` until the owner applies it, and no placeholder went into `Envelope.id`: `0` and `""` are values in the identifier space that name nothing, and a `q`-prefixed token is an id from another space in the field every command reads back. `Envelopes` carries a `queued` count instead, rendered under the table as *"N queued messages, see `himalaya pimdir queue list`"* and serialized for `--json`. Zero for every other backend, which is the truth about a backend whose writes reach the server as they are made, so the field stays least-common-denominator rather than pimdir trivia.

  `envelope search` reports none whatever the backend, because a queued creation is never matched against the query and a count the filter never saw would be worse than no count.

- **`himalaya pimdir queue list`** (capability `commands`), rendering a queued creation as mail. The `pimdir` binary is kind-agnostic and prints ids, hashes and flags on purpose; this client holds the conventions and the blobs, so it reads the flags, subject and recipient out of the action's own `v: 1` summary and shows when the row was queued. The row id is in a ROW column, named for what it is: it addresses a pending action, not a message, and the message gets a different id when the owner applies it.

- **`himalaya pimdir queue cancel <row>`**, through io-pimdir's scoped owner operation, confirming unless `--yes`. It is the only retraction a queued creation has: a staged flag or move is undone by doing the opposite, `set-flags` being absolute rather than a delta, but a message that does not exist yet cannot be deleted. A sync in flight owns the store, and that is reported as a sync running with the action possibly already applied, rather than as a lock error.

- **`message save` is unchanged.** The shared commands do not vary their wording by backend.

## What did not land

`Envelope.id: Option<String>` — whether a queued creation belongs in `envelope list` at all is a v1 question, and widening the shared envelope for every backend before the drafts UX says it must is not worth it. If the answer turns out to be yes, `null` is the encoding, and it is an additive change at that point.

Draining, again, and it is worth writing down why: the queue carries no `source` column, the drainer stamps its own, and bindings are keyed `(collection, link_id, source)`. Himalaya has no source to stamp since `pimdir-producer-reader` replaced `pimdir.source` with `pimdir.account`, so it cannot drain correctly even in principle.

## A defect this found upstream

io-pimdir's overlaid page could come back short in the middle of a collection, because a staged removal dropped a row the statement had returned. `scan_items` pages until a short page, as every consumer of a keyset page does, so one staged deletion would have ended a whole-collection scan early and silently dropped every message past it. Fixed upstream in io-pimdir's `overlay-page-is-total` before this landed; this change depends on it.

## Tests

src/pimdir/backend.rs: a queued creation renders as mail with an empty id, its subject, recipient and Message-ID read from the staged summary; and only a creation renders in that view, a staged removal having nothing to show there since it addresses a message the listing already reflects.

118 tests pass.

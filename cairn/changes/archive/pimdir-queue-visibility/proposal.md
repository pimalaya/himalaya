---
cairn: change
id: pimdir-queue-visibility
status: landed
created: 2026-08-27
---

# A staged write is invisible until Neverest runs

## Why

`pimdir-producer-reader` made Himalaya a reader and a producer, which was right, and left one consequence unaddressed: a write is an action on the queue, and a read is a projection of the committed index, and nothing connects the two. Flag a message and the flag is gone from the listing. Move one and it stays put. Nothing is lost, and the change lands on the next sync, but for the length of that window Himalaya reports a store that disagrees with what the user just did. Read as a bug it looks like data loss, which is the worst reading available for a mail client over a store measured in gigabytes.

The format anticipated this. SPEC §15.4 lets a reader overlay a collection's pending actions on its projection for exactly this reason, and `PimdirProducer::pending_actions` already hands out the rows. Himalaya never called it.

Draining is not the answer, and it is worth writing down why so it is not proposed again. The queue carries no source column: the drainer stamps its own (`stage_action` takes the source from a `PimdirSourceStore`), and bindings are keyed `(collection, link_id, source)`. A drain by anything other than the sync engine stages the change against a source nothing pushes, silently. Himalaya has no source to stamp since `pimdir.source` was removed, so it cannot drain correctly even in principle. It stays a reader and a producer.

## What

- Reads go through the overlaying `PimdirReader`, so a staged `set-flags`, `remove`, `move`, `copy` or `update` shows immediately. All five keep the item's `seq`, so nothing changes about how a message is addressed and `Envelope.id` stays a `String`.
- A queued create is not shown as a message. It has no `seq` until Neverest applies it, so there is no id to put in an envelope, and inventing one (`0`, an empty string, a `q`-prefixed token) puts a value that names nothing into the field every command reads back. `add_message` already returns the link id it staged, which is what identifies a create across the window.
- Instead the listing says so: a mailbox with queued creates prints how many under the table and names where to see them. That is the moment the user is confused, not the moment they saved, so this is what actually prevents the issue report.
- `himalaya pimdir queue list` renders those creates as mail. The operator CLI is kind-agnostic and prints ids, hashes and flags on purpose; Himalaya holds the blobs and the conventions, so it can show a sender, a subject and an age where `pimdir` can only show `a3f9…`.
- `himalaya pimdir queue cancel <id>` retracts one, through io-pimdir's scoped owner operation. Fail-fast during a sync is correct and the message says so rather than surfacing a lock error.
- `message save` keeps its generic confirmation. The shared commands do not vary their wording by backend.

## Deferred

Whether a queued create belongs in `message list` at all is a v1 question. If the answer turns out to be yes, the change is `Envelope.id: Option<String>` with `null` meaning no id assigned yet, which is the honest encoding and an additive one. It is not worth widening the shared envelope for every backend before the drafts UX says it must.

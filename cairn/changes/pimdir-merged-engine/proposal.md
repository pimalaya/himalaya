---
cairn: change
id: pimdir-merged-engine
status: landed
created: 2026-09-03
---

# The pimdir backend over the merged io-pimdir

## Why

io-replica is retired: its sync engine now lives inside io-pimdir, which implements both the storage and the sync parts of the pimdir standard. Himalaya's pimdir backend still built against io-pimdir 0.3 plus io-replica, through two git patches, and against a store shape the merged crate no longer writes.

Two things about that shape were Himalaya's to carry and are now the library's. The `v: 1` meta was an opaque JSON string Himalaya deserialised into a private view to build an envelope, guessing at fields the annex may or may not have written, and folding a whole address list into one bare email. And the link id, meta and sort key of a message being added went through `io_pimdir::conventions`, a module that no longer exists.

## What

The backend reads typed summaries. A read joins the item's `mail_summary` row and its addresses, so an envelope carries every `From` and `To` address with its display name, the `In-Reply-To` list, the date, the size and whether a part is an attachment, straight from the fields the standard names, with no JSON to parse.

A listing takes the store's own newest-first order, the mail sort key being the date, rather than re-sorting a link-id scan in memory.

An added message derives its link id through io-pimdir's mail derivation, `summary::mail::derive`, the same call the owner makes on the same body when it applies the action. The queued `Add` carries no summary any more, so `pimdir queue list` derives the row it shows from the body the action pins.

A mailbox is a collection declared `message/rfc822`. The allowance for a kind-less collection, which a sync predating declared kinds left, goes: such a store predates the merged format and is refused on open, the library saying to delete it and resync.

The `pimdir` feature drops `dep:io-replica`, io-pimdir moves to 0.4 through a path patch on the local checkout until it is released, and `mail-parser` stays: the shared attachment, search and compose code the `backend` cfg compiles in still parses bodies with it.

## Not in scope

A store written by io-pimdir 0.3 gets no migration here or in the library. The draft schema offered none, and Neverest recreates the store on the next sync.

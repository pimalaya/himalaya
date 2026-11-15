---
cairn: change
change: duplicate-link-id-mints-an-item
---

# Delta

## ADDED Requirements

### Requirement: A Message-ID is not an address
The pimdir backend SHALL NOT assume an item's link id is the `Message-ID` its body carries, nor that a `Message-ID` identifies at most one message in a mailbox. A store may hold two messages of one mailbox sharing a `Message-ID`, keyed apart by the store (pimdir SPEC §9), and both SHALL list, read and act as ordinary messages.

What stays unique is the key and the public id: `(collection, link_id)` still names one item and `seq` still names one message. What ends is the link id being derivable from the body, so a read that re-derives an identity in order to address a row is addressing an unknown number of them.

A mailbox holding one `Message-ID` twice is ordinary (a double delivery, a retried append, a restore, a copy of a sent message), and the store now keeps both rather than one. Showing one of them, or resolving an identity to whichever row came first, hides a message the server holds.

#### Scenario: A duplicated message lists twice
- GIVEN a mailbox whose store holds two items whose bodies carry one `Message-ID`
- WHEN the backend lists the mailbox
- THEN both appear, each with its own public id, and neither is marked

## MODIFIED Requirements

### Requirement: pimdir shows a short public id
The pimdir backend SHALL show and accept each message's public id (`items.seq`, a small store-assigned integer, the same across every mailbox the message is filed in) as its `Envelope.id`, not the internal `link_id`. It SHALL check the id against the collection before reading a body or staging an action, and SHALL fail clearly on a non-numeric or unknown one. `add_message` SHALL return the link id it staged: a queued create has no `seq` yet, the store assigning one when its owner applies the action.

Addressing by the public id is what keeps two duplicated messages distinguishable: they carry one `Message-ID` between them and have two `seq`s, so an address derived from the body would be ambiguous where a `seq` is not.

### Requirement: pimdir is a reader and a producer, never the owner
The pimdir backend SHALL treat the store as a possibly-partial cache owned by the sync engine. `get_message` on an item whose body is not local (`level < Full`, no stored object) SHALL report a clear "body not fetched" state (the cue to sync), not a data-loss error; the item still lists.

Reads SHALL go through `PimdirReader`, the role that takes no lock (pimdir SPEC §8) and carries no write at all, so a sync in flight neither blocks Himalaya nor is blocked by it, and the backend cannot drain the queue or sweep the store even by mistake.

The reader SHALL overlay the queue (pimdir SPEC §15.4), so an action this client staged is visible on the next read rather than on the next sync: a staged `set-flags`, `update`, `remove`, `move` or `copy` changes what a listing shows. Each addresses a message that already exists and keeps its public id, so a staged write never changes how a message is addressed.

A write SHALL be staged as a queued `PimdirAction` through a producer handle (`store_flags`→`SetFlags`, `add_message`→`Add`, `copy_messages`→`Copy`, `move_messages`→`Move`, `delete_messages`→`Remove`), addressed by the public `seq`, for the store's owner to apply and push. The backend SHALL NOT write the index, load a collection, or run the owner's object sweep: a sweep run beside a sync destroys the bodies it has streamed but not yet attached, which SPEC §14 explicitly invites it to leave pending. A body an action references SHALL be written to the blob store durably before the action is enqueued, the queue row being what pins it.

`SetFlags` carries the whole replacement set, so applying it twice lands the same state; a set the store reports as unknown contributes no markers rather than staging an unknown one, which would erase what a sync knows. pimdir has no native trash.

An added message SHALL derive its link id, summary and sort key through `io_pimdir::conventions`, the one implementation of SPEC Annex A, which is the bare `Message-ID` with nothing prepended. A staged `Add` whose link id the collection already holds SHALL park (pimdir SPEC §15.3): it neither deduplicates against the stored copy nor mints a second key. Minting is the store's answer to what a source hands over; parking is its answer to a producer authoring a message the collection already has.

## REMOVED Requirements

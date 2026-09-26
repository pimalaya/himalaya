---
cairn: delta
change: pimdir-merged-engine
---

## MODIFIED Requirements

### Requirement: Local storage backends
Maildir, m2dir and pimdir SHALL adapt io-maildir, io-m2dir and io-pimdir. Maildir stores added messages under `cur/` and SHALL read an entry's flags through io-maildir rather than parsing the filename itself, so the meaning of a Maildir name is decided in the library that owns the format. m2dir is content-addressed with no native copy or move, so those are a get plus a store (plus a delete for move), and its flags live in a `.meta/<id>.flags` sidecar. m2dir mailbox `rename` and message `copy`/`move` remain unavailable until io-m2dir supports them. pimdir is an offline cache the sync engine populates, io-pimdir implementing both the store and the engine: reads project the store's items with their typed mail summaries (pimdir STORAGE Annex A) without body reads, and writes are actions queued for the store's owner to apply and a later sync to propagate, never direct SQL.

### Requirement: A pimdir mailbox is its collection id
The pimdir backend SHALL show and accept a mailbox as the store's collection id, verbatim: the collection `imap/INBOX` is the mailbox `imap/INBOX`. It SHALL NOT derive, strip or accept a shortened spelling, and no configuration SHALL offer one.

The id is opaque to the store, which neither parses nor validates it (pimdir SPEC 9.2) and models hierarchy through `parent` rather than through a separator. Any shortening is therefore a guess at the producer's convention, and one that makes a single mailbox answer to two spellings. This is the JMAP backend's shape, whose ids are opaque server strings, and `[mailbox.alias]` is the shortcut for both.

`Mailbox.name` SHALL carry the collection row's own name rather than one derived from the id.

A mailbox is a collection declared `message/rfc822`, matched on the bare media type. A collection declaring no kind is not a mailbox: the store cannot summarise it, and the draft that left kinds undeclared is refused on open rather than read.

A mailbox matching no collection of the account SHALL be refused naming the ones it holds. It SHALL NOT be passed to the store unresolved, which reads as a mailbox that exists and is empty.

### Requirement: pimdir is a reader and a producer, never the owner
The pimdir backend SHALL treat the store as a possibly-partial cache owned by the sync engine. `get_message` on an item whose body is not local (`level < Full`, no stored object) SHALL report a clear "body not fetched" state (the cue to sync), not a data-loss error; the item still lists.

Reads SHALL go through `PimdirReader`, the role that takes no lock (pimdir SPEC §8) and carries no write at all, so a sync in flight neither blocks Himalaya nor is blocked by it, and the backend cannot drain the queue or sweep the store even by mistake.

An envelope SHALL be built from the item's typed mail summary and the addresses joined to it (pimdir STORAGE Annex A.1, A.6): every `From` and `To` address with its display name, the `In-Reply-To` list, the date, the size and the attachment flag, with no body read. A listing SHALL take the store's own order, newest first, the mail sort key being the date.

The reader SHALL overlay the queue (pimdir SPEC §15.4), so an action this client staged is visible on the next read rather than on the next sync: a staged `set-flags`, `update`, `remove`, `move` or `copy` changes what a listing shows. Each addresses a message that already exists and keeps its public id, so a staged write never changes how a message is addressed.

A write SHALL be staged as a queued `PimdirAction` through a producer handle (`store_flags`→`SetFlags`, `add_message`→`Add`, `copy_messages`→`Copy`, `move_messages`→`Move`, `delete_messages`→`Remove`), addressed by the public `seq`, for the store's owner to apply and push. The backend SHALL NOT write the index, load a collection, or run the owner's object sweep: a sweep run beside a sync destroys the bodies it has streamed but not yet attached, which SPEC §14 explicitly invites it to leave pending. A body an action references SHALL be written to the blob store durably before the action is enqueued, the queue row being what pins it.

`SetFlags` carries the whole replacement set, so applying it twice lands the same state; a set the store reports as unknown contributes no markers rather than staging an unknown one, which would erase what a sync knows. pimdir has no native trash.

An added message SHALL derive its link id through io-pimdir's mail derivation (`summary::mail::derive`), the one implementation of SPEC Annex A.1, which is the bare `Message-ID` with nothing prepended, and SHALL name it on the queued `Add`. The action carries no summary: the owner derives the summary and the sort key from the body when it applies the action, through the same call, so the two never disagree. A staged `Add` whose link id the collection already holds SHALL park (pimdir SPEC §15.3): it neither deduplicates against the stored copy nor mints a second key. Minting is the store's answer to what a source hands over; parking is its answer to a producer authoring a message the collection already has.

### Requirement: The pimdir subcommand reads and retracts the queue
Himalaya SHALL carry a `pimdir` subcommand for what the operator CLI cannot do without knowing mail. `queue list` SHALL render a queued creation as a message (flags, subject, recipient, and when it was queued, from the row's `created_at`) where the kind-agnostic `pimdir` binary can only print ids and hashes. The queued action carries no summary, so the row SHALL be derived from the body the action pins, through the same derivation the owner applies; an action pinning no body renders with its flags alone. `queue cancel` SHALL retract one row through io-pimdir's scoped owner operation, confirming first unless `--yes`.

Taking the owner role briefly is what cancelling costs (pimdir SPEC §15.5); the backend read and write paths SHALL NOT reach it. A store another process owns SHALL be refused immediately, saying a sync is running and that the action may already have been applied.

#### Scenario: A queued creation is rendered from its body
- GIVEN a queued `add` pinning a body whose headers carry a subject and a recipient
- WHEN the queue is listed
- THEN the row shows that subject and recipient, and an empty message id

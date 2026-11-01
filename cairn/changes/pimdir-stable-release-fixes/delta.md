---
cairn: delta
change: pimdir-stable-release-fixes
---

## ADDED Requirements

### Requirement: A pimdir refile or delete stages the ids it found
`copy_messages`, `move_messages` and `delete_messages` SHALL check every given id against the mailbox before staging the first row, skip an id the mailbox does not hold, stage one row per id found and report how many they found (commands, "A command given message ids says how many it found"). A call finding none SHALL stage nothing. A malformed id SHALL fail the call before anything is staged.

#### Scenario: One id of two exists
- GIVEN a pimdir mailbox holding message 3 and no message 999
- WHEN `message copy --from imap/INBOX --to imap/Trash 3 999` runs
- THEN one `copy` row is queued for 3, and the command succeeds reporting one message

### Requirement: The pimdir queue view shows parked rows
`himalaya pimdir queue list` SHALL list the mailbox's parked rows, any action kind, beside its queued creations and sends: the row id, the action kind, when it was queued, and the error that parked it, as `parked` under `--json`. `queue cancel` SHALL take a parked row as it takes a pending one.

#### Scenario: A duplicate draft parks
- GIVEN a staged `add` the sync engine parked because the mailbox already holds its `Message-ID`
- WHEN `himalaya pimdir queue list -m imap/Drafts` runs
- THEN the row is listed as parked with its error, and `pimdir queue cancel` on its id removes it

## MODIFIED Requirements

### Requirement: pimdir is a reader and a producer, never the owner
The pimdir backend SHALL treat the store as a possibly-partial cache owned by the sync engine. `get_message` on an item whose body is not local (`level < Full`, no stored object, or a stored object whose blob file is gone, pimdir GUIDE §14) SHALL fail with the code `body-pending` (commands, "A JSON error carries a stable code"), the cue to sync, not a data-loss error; the item still lists.

Reads SHALL go through `PimdirReader`, the role that takes no lock (pimdir STORAGE §8) and carries no write at all, so a sync in flight neither blocks Himalaya nor is blocked by it, and the backend cannot drain the queue or sweep the store even by mistake.

An envelope SHALL be built from the item's typed mail summary and the addresses joined to it (pimdir STORAGE Annex A.1, A.6): every `From` and `To` address with its display name, the `In-Reply-To` list, the date, the size and the attachment mark as stored, with no body read. `hasAttachment` SHALL be `true` or `false` as the store marks it, and `null` on a row that holds no mark. A listing SHALL take the store's own order, newest first, the mail sort key being the date.

The reader SHALL overlay the queue (pimdir STORAGE §15.4), so an action this client staged is visible on the next read rather than on the next sync: a staged `set-flags`, `update`, `remove`, `move` or `copy` changes what a listing shows. Each addresses a message that already exists and keeps its public id, so a staged write never changes how a message is addressed.

A write SHALL be staged as a queued `PimdirAction` through a producer handle (`store_flags`→`SetFlags`, `add_message`→`Add`, `copy_messages`→`Copy`, `move_messages`→`Move`, `delete_messages`→`Remove`), addressed by the public `seq`, for the store's owner to apply and push. The backend SHALL NOT write the index, load a collection, or run the owner's object sweep: a sweep run beside a sync destroys the bodies it has streamed but not yet attached, which STORAGE §14 explicitly invites it to leave pending. A body an action references SHALL be written to the blob store durably before the action is enqueued, the queue row being what pins it.

`SetFlags` carries the whole replacement set, so applying it twice lands the same state. An add or a remove on a set the store reports as unknown SHALL be refused, saying the message's flags were never synced: it has no markers to build on, and the replacement would erase whatever the server holds. A `set` replaces the whole set, so it SHALL stage one regardless. A change whose replacement set equals the set the message shows SHALL stage nothing for it, the message still counting as found. pimdir has no native trash.

An added message SHALL derive its link id through io-pimdir's mail derivation (`summary::mail::derive`), the one implementation of STORAGE Annex A.1, which is the bare `Message-ID` with nothing prepended, and SHALL name it on the queued `Add`. The action carries no summary: the owner derives the summary and the sort key from the body when it applies the action, through the same call, so the two never disagree. A staged `Add` whose link id the collection already holds SHALL park (pimdir STORAGE §15.3): it neither deduplicates against the stored copy nor mints a second key. Minting is the store's answer to what a source hands over; parking is its answer to a producer authoring a message the collection already has.

#### Scenario: An add on flags never synced is refused
- GIVEN a pimdir message whose flags the sync engine never read
- WHEN `himalaya flag add -m imap/INBOX -f flagged 42` runs
- THEN the command fails saying the flags of `42` were never synced, and nothing is queued

#### Scenario: A flag the message carries already
- GIVEN a pimdir message showing `\Seen`
- WHEN `himalaya flag add -m imap/INBOX -f seen 6` runs
- THEN no row is queued, and the command reports one message found

#### Scenario: A body purged under a held item
- GIVEN a pimdir item whose stored object's blob file is gone
- WHEN `message read` runs on it
- THEN the command fails with the code `body-pending`

### Requirement: pimdir writes name their queue row
`pimdir message add` and `pimdir message send` SHALL stage a message as the shared `message add` and `message send` do, and print the queue row it waits in with the message's bare `Message-ID`: `{"queueId": i64, "messageId": string}` for an add, plus `"copy": bool` (the `submit` row carries the copy) and `"copyQueueId": i64` (a copy queued beside the send, for an owner declaring no capabilities) for a send. `pimdir flag add|set|remove` SHALL print the row of each message they found: `{"queued": [{"id": string, "queueId": i64 | null}]}`, `null` for a message the change left as it was, for which nothing was staged. The shared commands SHALL keep printing no queue detail.

`pimdir queue show <ROW>` SHALL say where a row of the account's queue stands, read through io-pimdir's `action_status` (pimdir STORAGE §15.4): `{"queueId", "state": "pending" | "parked" | "applied" | "unknown", "collection"?, "kind"?, "attempts"?, "error"?, "appliedAt"?, "seq"?}`. A pending or parked row carries `collection`, `kind` and `attempts`, a parked one `error` too; an applied one carries `collection`, `appliedAt` and, for an add, `seq`, the public id of the message it created. A row neither queued nor in the store's receipts (kept seven days), or one anchored on another account's collection, SHALL read as `unknown`: cancelled, or applied long ago. A row the sync engine performed rather than applied, a message sent or a mailbox created, reads as `applied` without `seq`, its receipt recorded on acknowledgement (pimdir STORAGE §15.5).

The plain output of every `pimdir` subcommand SHALL end with a newline.

#### Scenario: A staged draft is followed by its row
- GIVEN a pimdir account
- WHEN `himalaya --json pimdir message add -m imap/Drafts` stages a message
- THEN it prints `{"queueId":12,"messageId":"d1@x.org"}`, and `pimdir queue show 12` prints `"state":"pending"` until the sync engine applies the row, then `"state":"applied"` with the `seq` the message is read by

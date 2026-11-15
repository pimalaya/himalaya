---
cairn: delta
change: pimdir-flag-commands
---

# Delta

## ADDED Requirements

### Requirement: pimdir stages flags by their raw spelling
`himalaya pimdir flag list|add|set|remove` SHALL read and stage the flags of a pimdir message by their raw spelling, verbatim. A value naming an IANA flag SHALL be accepted only in its canonical spelling, compared without case and staged in the canonical case, a bare `seen` or `$seen` being refused naming `\Seen`; `\Deleted` and `\Recent` SHALL be refused. A backslash names a system flag, so one in front of any other word SHALL be refused, as SHALL a value that is empty or holds whitespace, a control character, a character outside ASCII or one IMAP forbids in a flag (`(`, `)`, `{`, `%`, `*`, `"`, `]`, `\`). Any other value is a keyword.

`add`, `set` and `remove` SHALL fail when no message is named or none of the named ones exists, and SHALL stage one `SetFlags` per message through the backend's `stage_flags`, checked against the sources' declarations (pimdir STORAGE §15.6): a source declaring `mail.flags.keywords` as `none` SHALL refuse a keyword change with its detail, and one declaring it in part SHALL queue it with the declaration's warning as it is. `list` SHALL print the flags a message carries as the overlaying reader shows them, and `null` under `--json` for a message whose flags the sync engine never read.

An add or a remove SHALL match the flags a message carries as flags, not as spellings: a marker the store holds as `\seen`, or as Thunderbird's `Junk`, is the one `\Seen` or `$Junk` keeps on an add and drops on a remove. This holds for the shared flag commands on a pimdir account, which stage through the same path.

#### Scenario: A keyword is staged and read back
- GIVEN a pimdir account whose `imap` source declares `mail.flags.keywords`
- WHEN `himalaya --json pimdir flag add -m imap/INBOX -f act 42` runs
- THEN it prints `{"queued":[{"id":"42","queueId":7}]}`, and `pimdir flag list -m imap/INBOX 42` lists `act` before the next sync

#### Scenario: A source without keywords refuses one
- GIVEN a pimdir account whose `msgraph` source declares `mail.flags.keywords` as `none`
- WHEN `himalaya pimdir flag add -m msgraph/Inbox -f act 42` runs
- THEN the command fails naming `mail.flags.keywords`, the `msgraph` source and its detail, and nothing is queued

#### Scenario: A flag held in another spelling is removed
- GIVEN a pimdir message carrying `\seen` and `Junk`
- WHEN `himalaya pimdir flag remove -m imap/INBOX -f '\Seen' -f '$Junk' 42` runs
- THEN the staged set holds neither

#### Scenario: A bare IANA name is refused
- WHEN `himalaya pimdir flag add -m imap/INBOX -f seen 42` runs
- THEN the command fails naming `\Seen`, and nothing is queued

### Requirement: pimdir stages a flag change on every message or none
`stage_flags`, which the shared `store_flags` goes through, SHALL check every message of a call the mailbox holds against the sources' declarations before it queues the first row, so a call the check refuses on any message queues nothing. An id the mailbox does not hold SHALL be skipped, the call staging the others and reporting how many it found (commands, "A command given message ids says how many it found"); a malformed id SHALL fail the call, as it does on IMAP. Each row is still its own transaction: an enqueue failing after the check leaves the rows queued before it. This holds for the shared `flag add|set|remove` on a pimdir account as for `pimdir flag`.

#### Scenario: A refusal part-way stages nothing
- GIVEN a pimdir account whose `msgraph` source declares `mail.flags.keywords` as `none`, and message 41 of `msgraph/Inbox` carries the keyword `act`
- WHEN `himalaya flag set -m msgraph/Inbox -f seen 40 41 42` runs, the set dropping `act` from 41 only
- THEN the command fails naming `mail.flags.keywords` for message 41, and no row is queued for 40, 41 or 42

## MODIFIED Requirements

### Requirement: pimdir is a reader and a producer, never the owner
The pimdir backend SHALL treat the store as a possibly-partial cache owned by the sync engine. `get_message` on an item whose body is not local (`level < Full`, no stored object) SHALL fail with the code `body-pending` (commands, "A JSON error carries a stable code"), the cue to sync, not a data-loss error; the item still lists.

Reads SHALL go through `PimdirReader`, the role that takes no lock (pimdir SPEC §8) and carries no write at all, so a sync in flight neither blocks Himalaya nor is blocked by it, and the backend cannot drain the queue or sweep the store even by mistake.

An envelope SHALL be built from the item's typed mail summary and the addresses joined to it (pimdir STORAGE Annex A.1, A.6): every `From` and `To` address with its display name, the `In-Reply-To` list, the date, the size and the attachment mark as stored, with no body read. `has-attachment` SHALL be `true` or `false` as the store marks it, and `null` on a row that holds no mark. A listing SHALL take the store's own order, newest first, the mail sort key being the date.

The reader SHALL overlay the queue (pimdir SPEC §15.4), so an action this client staged is visible on the next read rather than on the next sync: a staged `set-flags`, `update`, `remove`, `move` or `copy` changes what a listing shows. Each addresses a message that already exists and keeps its public id, so a staged write never changes how a message is addressed.

A write SHALL be staged as a queued `PimdirAction` through a producer handle (`store_flags`→`SetFlags`, `add_message`→`Add`, `copy_messages`→`Copy`, `move_messages`→`Move`, `delete_messages`→`Remove`), addressed by the public `seq`, for the store's owner to apply and push. The backend SHALL NOT write the index, load a collection, or run the owner's object sweep: a sweep run beside a sync destroys the bodies it has streamed but not yet attached, which SPEC §14 explicitly invites it to leave pending. A body an action references SHALL be written to the blob store durably before the action is enqueued, the queue row being what pins it.

`SetFlags` carries the whole replacement set, so applying it twice lands the same state. An add or a remove on a set the store reports as unknown SHALL be refused, saying the message's flags were never synced: it has no markers to build on, and the replacement would erase whatever the server holds. A `set` replaces the whole set, so it SHALL stage one regardless. pimdir has no native trash.

An added message SHALL derive its link id through io-pimdir's mail derivation (`summary::mail::derive`), the one implementation of SPEC Annex A.1, which is the bare `Message-ID` with nothing prepended, and SHALL name it on the queued `Add`. The action carries no summary: the owner derives the summary and the sort key from the body when it applies the action, through the same call, so the two never disagree. A staged `Add` whose link id the collection already holds SHALL park (pimdir SPEC §15.3): it neither deduplicates against the stored copy nor mints a second key. Minting is the store's answer to what a source hands over; parking is its answer to a producer authoring a message the collection already has.

#### Scenario: An add on flags never synced is refused
- GIVEN a pimdir message whose flags the sync engine never read
- WHEN `himalaya flag add -m imap/INBOX -f flagged 42` runs
- THEN the command fails saying the flags of `42` were never synced, and nothing is queued

### Requirement: pimdir writes name their queue row
`pimdir message add` and `pimdir message send` SHALL stage a message as the shared `message add` and `message send` do, and print the queue row it waits in with the message's bare `Message-ID`: `{"queueId": i64, "messageId": string}` for an add, plus `"copy": bool` (the `submit` row carries the copy) and `"copyQueueId": i64` (a copy queued beside the send, for an owner declaring no capabilities) for a send. `pimdir flag add|set|remove` SHALL print the row of each message they stage: `{"queued": [{"id": string, "queueId": i64}]}`. The shared commands SHALL keep printing no queue detail.

`pimdir queue show <ROW>` SHALL say where a row of the account's queue stands, read through io-pimdir's `action_status` (pimdir STORAGE §15.4): `{"queueId", "state": "pending" | "parked" | "applied" | "unknown", "collection"?, "kind"?, "attempts"?, "error"?, "appliedAt"?, "seq"?}`. A pending or parked row carries `collection`, `kind` and `attempts`, a parked one `error` too; an applied one carries `collection`, `appliedAt` and, for an add, `seq`, the public id of the message it created. A row neither queued nor in the store's receipts (kept seven days), or one anchored on another account's collection, SHALL read as `unknown`: cancelled, or applied long ago. A row the sync engine performed rather than applied, a message sent or a mailbox created, reads as `applied` without `seq`, its receipt recorded on acknowledgement (pimdir STORAGE §15.5).

#### Scenario: A staged draft is followed by its row
- GIVEN a pimdir account
- WHEN `himalaya --json pimdir message add -m imap/Drafts` stages a message
- THEN it prints `{"queueId":12,"messageId":"d1@x.org"}`, and `pimdir queue show 12` prints `"state":"pending"` until the sync engine applies the row, then `"state":"applied"` with the `seq` the message is read by

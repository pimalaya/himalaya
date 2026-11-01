---
cairn: delta
change: pimdir-follows-storage-draft-08
---

## ADDED Requirements

None: every behaviour this change moves already has its requirement.

## MODIFIED Requirements

### Requirement: A pimdir store failure carries its code (backends)
A failure io-pimdir reports SHALL keep io-pimdir's message as a cause and carry, under `--json`, the code `PimdirError::code()` gives it, spelled the same, when it is one a caller acts on: `store-version`, `store-uncreated`, `store-foreign`, `store-corrupt`, `store-not-current`, `store-hash-algo`, `sync-version`, `store-busy`, `store-owned`, `store-staging`, `store-in-flight`, `store-exposed`, `item-duplicate`, `flag-invalid`, `action-malformed`, `conflict-unread`, `unsupported`, `no-performer` and `ambiguous-performer`. An SQL, I/O or JSON failure carries none. Himalaya SHALL NOT derive a code from a message, and SHALL NOT advise deleting the store.

A store of another draft of the format (pimdir STORAGE §4.2), earlier or later, or one naming no format version, is `store-version`: drafts are exact, so it opens under no other draft and nothing reconciles or migrates it. Its message SHALL say to upgrade Himalaya when the store's sync engine is newer, else to reinstall the store as neverest's MIGRATION.md says, which syncs out the writes still queued in it before moving it aside. A store whose directory or pimdir.db another user can read or write, or a link where a lock file belongs, is `store-exposed`, refused before anything in it is read, its message saying to restrict it.

#### Scenario: A store another draft wrote
- GIVEN a pimdir store whose `store_meta.format_version` names another draft than Himalaya's io-pimdir follows
- WHEN any pimdir command opens it
- THEN the command fails with the code `store-version`, naming the draft, saying to upgrade Himalaya or reinstall the store as neverest's MIGRATION.md says, and never to delete it

#### Scenario: A store others can read
- GIVEN a pimdir store whose directory is mode `0755`
- WHEN any pimdir command opens it
- THEN the command fails with the code `store-exposed`, saying to restrict it

#### Scenario: A draft the mailbox holds already
- GIVEN a pimdir mailbox holding a message whose `Message-ID` is `d1@x.org`
- WHEN `himalaya message add -m imap/Drafts` stages a message with that `Message-ID`
- THEN the command fails with the code `item-duplicate`, and nothing is queued

### Requirement: The pimdir subcommand reads and retracts the queue (backends, the cancel paragraph and its scenario)
Taking the owner role briefly is what cancelling costs (pimdir STORAGE §15.5); the backend read and write paths SHALL NOT reach it. The cancel SHALL open the store only as it stands (io-pimdir `open_current`): it SHALL NOT create, migrate nor reconcile the schema. A store of another draft SHALL be refused as `store-version` and a damaged one as `store-corrupt`, each left as it is. A store another process owns SHALL be refused immediately, saying a sync is running and that the action may already have been applied.

#### Scenario: A cancel on a store of another draft
- GIVEN a store whose `store_meta.format_version` names another draft
- WHEN `himalaya pimdir queue cancel` runs on one of its rows
- THEN the command fails with the code `store-version`, pointing at neverest's MIGRATION.md, and neither the store's stamp nor the row changes

### Requirement: pimdir writes name their queue row (backends)
`pimdir message add` and `pimdir message send` SHALL stage a message as the shared `message add` and `message send` do, and print the queue row it waits in with the message's bare `Message-ID`: `{"queueId": i64, "messageId": string}` for an add, plus `"copy": bool` (the `submit` row carries the copy) and `"copyQueueId": i64` (a copy queued beside the send, when its performer declares no capabilities) for a send. `pimdir flag add|set|remove` SHALL print the row of each message they found: `{"queued": [{"id": string, "queueId": i64 | null}]}`, `null` for a message the change left as it was, for which nothing was staged. The shared commands SHALL keep printing no queue detail.

### Requirement: pimdir stages flags by their raw spelling (backends, the first paragraph and a scenario)
`himalaya pimdir flag list|add|set|remove` SHALL read and stage the flags of a pimdir message by their raw spelling, verbatim. A value naming an IANA flag SHALL be accepted only in its canonical spelling, compared without case and staged in the canonical case, a bare `seen` or `$seen` being refused naming `\Seen`; `\Deleted` and `\Recent` SHALL be refused. A backslash names a system flag, so one in front of any other word SHALL be refused, as SHALL a character IMAP forbids in a flag (`(`, `)`, `{`, `%`, `*`, `"`, `]`, `\`). A value the store cannot hold (pimdir STORAGE §13), empty or holding whitespace, a control character or a character outside ASCII, SHALL be refused with the code `flag-invalid`, the code io-pimdir's enqueue gives a flag set holding one, and nothing staged. Any other value is a keyword.

#### Scenario: A flag the store cannot hold
- GIVEN any pimdir account
- WHEN `himalaya --json pimdir flag add -m imap/INBOX -f 'to do' 42` runs
- THEN the command exits 1 with the code `flag-invalid`, and nothing is queued

### Requirement: pimdir sends by queueing a submit intent (backends, the payload paragraph and a scenario)
The payload SHALL be built by io-pimdir's `PimdirSubmit` (STORAGE Annex B.2), `v: 1` JSON carrying `source`, `from`, `rcpts`, `subject`, and `copy` when the intent carries the copy, the keys and values a performer reads back with `PimdirSubmit::from_payload`. The message is the row's body (`queue.object_hash`), never a member of the payload. `source` SHALL name the performer: the declared one, resolved as `pimdir performer` shows it, else, when the sources syncing the anchor declare nothing, the one source syncing it. An anchor no source syncs yet SHALL be refused with `no-performer`, and one several undeclared sources sync with `ambiguous-performer`, nothing staged. The envelope is derived from the headers by the same function the SMTP adapter uses: the first `From:` address, every `To:`, `Cc:` and `Bcc:` address, and the decoded subject. A message with no `From:` or no recipient SHALL be refused before anything is staged. The body SHALL be stored as given, `Bcc:` included; removing it is the sending channel's job.

#### Scenario: A mailbox no source syncs yet
- GIVEN a pimdir mailbox the sync engine declared and no source syncs yet
- WHEN a message is sent anchored on it
- THEN the command fails with the code `no-performer`, and nothing is queued

### Requirement: A JSON error carries a stable code (commands, the code table)
The table loses `store-newer` and `store-stale`, which io-pimdir no longer raises, and gains `store-version`, `sync-version`, `store-exposed`, `flag-invalid`, `action-malformed`, `conflict-unread`, `index-version` and `index-zone-unknown`; `store-corrupt` also covers a schema that is not its draft's, and `store-not-current` a store holding no schema yet.

| Code | Raised when |
|---|---|
| `store-version` | a pimdir store is of another draft of the format; upgrade Himalaya, or reinstall the store as neverest's MIGRATION.md says |
| `store-corrupt` | a pimdir store's version records disagree, or its schema is not its draft's |
| `store-not-current` | a pimdir store holds no schema yet and the verb creates none; its sync engine creates it |
| `sync-version` | a pimdir store's sync state was written by another version of its sync engine |
| `store-exposed` | a pimdir store is a link or another user can read or write it; restrict it |
| `flag-invalid` | a flag a pimdir store cannot hold: empty, or holding whitespace, a control or a non-ASCII character; nothing is staged |
| `action-malformed` | a pimdir action's payload breaks its schema; nothing is staged |
| `conflict-unread` | a pimdir conflict holds a body nobody has read; fetch it first |
| `index-version` | a pimdir search index is of another draft or shape; its sync engine rebuilds one of an earlier draft |
| `index-zone-unknown` | the zone a pimdir search index reads floating times in is no IANA zone |

### Requirement: pimdir searches through the store's index (search)
The SEARCH query language Himalaya answers is search-draft-08 §8, where it was search-draft-05 §8; the rest of the requirement is unchanged.

### Requirement: A missing or outdated pimdir index is a coded error (search)
A store with no search index SHALL fail every index command with the code `index-missing`, an index of this draft another build, tokenizer or store built with `index-outdated`, and an index of another draft or shape (pimdir SEARCH §2) with `index-version`, each saying the store's sync engine builds or rebuilds it; one of another shape or a later draft is left as it is. None SHALL advise deleting anything, and Himalaya SHALL NOT rebuild any of them.

#### Scenario: An index of an earlier draft
- GIVEN a pimdir store whose `index_meta.format_version` is `search-draft-07`
- WHEN `himalaya --json pimdir search lunch` runs
- THEN the command exits 1 with the code `index-version`, and the index is left as it is

### Requirement: A refused pimdir query names its fault (search)
A query the SEARCH §8.4 rules refuse SHALL be refused whole with the code `query-refused`, nothing searched. A query past the bounds of SEARCH §8.3, more than 1,024 Unicode scalar values, more than 32 groups open at once or more than 128 terms, SHALL be refused so, its §8.4 code `limit-exceeded`. The plain output SHALL render the fault's span over the query with Himalaya's ariadne setup; the JSON error's message SHALL name the §8.4 code, the span as Unicode scalar offsets and what was expected there.

#### Scenario: A query past its bounds
- GIVEN any pimdir account with an index
- WHEN `himalaya --json pimdir search` runs a query of 129 terms
- THEN the command exits 1 with the code `query-refused` and a message naming `limit-exceeded` at the 129th term's span

## REMOVED Requirements

None: `store-newer` and `store-stale` leave the code table and the store failure requirement, both modified above.

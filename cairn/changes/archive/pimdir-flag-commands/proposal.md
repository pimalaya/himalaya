---
cairn: change
id: pimdir-flag-commands
status: landed
created: 2026-10-15
---

# Stage keywords on a pimdir message

## Why

A pimdir store carries every flag its sources read, keywords included: a neverest store synced from IMAP lists `{"raw": "NonJunk", "iana": null}` on most of an inbox. Nothing can write one back. The shared `flag add|set|remove` stay limited to the four flags every backend handles (#790), and the pimdir namespace has no flag command, where IMAP has `imap store` and m2dir has `m2dir flag`.

The backend is mostly there. `store_flags` stages each `Flag` by its raw spelling in one `SetFlags` per message, and io-pimdir names the capability a keyword change needs, `mail.flags.keywords`, which the producer checks per row before queueing it.

## What

`himalaya pimdir flag list|add|set|remove`, the shape of `m2dir flag`:

- `list -m <MAILBOX> <ID>` prints the flags the message carries, staged ones included, one raw spelling per row.
- `add`, `set` and `remove` take `-m <MAILBOX>`, repeatable `-f <FLAG>` and message ids, and print the queue row of each message, `{"queued": [{"id", "queueId"}]}`, as the other pimdir writes do. The backend stages through a new `stage_flags`, which returns the row ids it gets from `enqueue` and which the shared `store_flags` now wraps.
- A flag is taken verbatim, as in `imap store`. A value naming an IANA flag is accepted only in its canonical spelling: `-f '\Seen'` stages `\Seen`, and `-f seen` or `-f '$seen'` is refused naming it, since `pimdir message add -f seen` means `\Seen` and the reader would classify the keyword `seen` as read. `\Deleted` (a sync verb) and `\Recent` (not storable) are refused, as are a backslash before any other word, a value that is empty or holds whitespace or a control character, a character outside ASCII, and the characters IMAP forbids in a flag. A call naming no message is refused.
- `-m` resolves aliases and roles as the shared flag commands do, the inbox when omitted.
- A source declaring `mail.flags.keywords` as `none` (Graph: "Graph categories are not mapped") refuses with that detail; one declaring it in part queues with the declaration's warning as it is (IMAP: PERMANENTFLAGS; Gmail: `$Important` only). What the sync engine then does with a keyword it cannot map is its own.

`store_flags` also changes for the shared commands: today it checks and queues one message at a time, so a multi-id call refused part-way leaves the earlier messages staged. It will check every message before the first enqueue, so a refusal queues nothing.

m2dir needs nothing: `m2dir flag` already takes free-form flags.

## Non-goals

The shared `flag` commands still take only the four flags every backend handles, and the shared search grammar does not change; on pimdir they only gain the all-or-nothing staging above, flag matching, and the refusal of an add or a remove on flags never synced (asked in review).

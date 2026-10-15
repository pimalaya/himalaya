---
cairn: change
id: pimdir-flag-commands
status: active
created: 2026-10-15
---

# Stage keywords on a pimdir message

## Why

A pimdir store carries every flag its sources read, keywords included: a neverest store synced from IMAP lists `{"raw": "NonJunk", "iana": null}` on most of an inbox. Nothing can write one back. The shared `flag add|set|remove` stay limited to the four flags every backend handles (#790), and the pimdir namespace has no flag command, where IMAP has `imap store` and m2dir has `m2dir flag`.

The backend is mostly there. `store_flags` stages each `Flag` by its raw spelling in one `SetFlags` per message, and io-pimdir names the capability a keyword change needs, `mail.flags.keywords`, which the producer checks per row before queueing it.

## What

`himalaya pimdir flag list|add|set|remove`, the shape of `m2dir flag`:

- `list -m <MAILBOX> <ID>` prints the flags the message carries, staged ones included, one raw spelling per row.
- `add`, `set` and `remove` take `-m <MAILBOX>`, repeatable `-f <FLAG>` and message ids, and print the queue row of each message, `{"queued": [{"id", "queueId"}]}`, as the other pimdir writes do. `store_flags` returns the row ids it already gets from `enqueue`.
- A flag is taken verbatim, as in `imap store`. A value naming an IANA flag is accepted only in its canonical spelling: `-f '\Seen'` stages `\Seen`, and `-f seen` or `-f '$seen'` is refused naming it, since `pimdir message add -f seen` means `\Seen` and the reader would classify the keyword `seen` as read. `\Deleted` (a sync verb) and `\Recent` (not storable) are refused, as is a value that is empty or holds whitespace or a control character.
- `-m` resolves aliases and roles as `pimdir message add` does.
- Every message is checked before the first row is queued, so a refusal leaves nothing half staged. A source declaring `mail.flags.keywords` as `none` (Graph: "Graph categories are not mapped") refuses with that detail; one declaring it in part queues with the warning naming the limit (IMAP: PERMANENTFLAGS; Gmail: `$Important` only).

What neverest does with a keyword other than `$Important` on a Gmail source is the sync engine's to define. A manual run against Gmail, recorded under `cairn/spec/testing/`, will say whether the row parks or is dropped, and the warning will name the outcome.

m2dir needs nothing: `m2dir flag` already takes free-form flags.

## Non-goals

The shared `flag` commands, the shared search grammar and the shared `flag set`, which replaces the whole set on pimdir as on Maildir, do not change.

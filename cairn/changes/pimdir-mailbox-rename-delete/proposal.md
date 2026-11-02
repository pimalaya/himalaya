---
cairn: change
id: pimdir-mailbox-rename-delete
status: landed
created: 2026-11-02
---

# pimdir renames and deletes a mailbox through the sync engine

## Why

A pimdir account can create a mailbox (pimdir-mailbox-create) but neither rename nor delete one: the store mirrors its server, and renaming or deleting the collection in the store would only diverge from it. pimdir b63b39a adds the `collection-rename` and `collection-delete` intents (STORAGE Annex B.2) and their capabilities `collection.rename` and `collection.delete`, performed by the source on its server, the change arriving with the next sync; io-pimdir 908db2b implements them.

## What

- `pimdir mailbox rename <MAILBOX> <NAME>`, queueing `{v: 1, source, name}` anchored on the mailbox, and `pimdir mailbox delete <MAILBOX>`, queueing `{v: 1, source}` anchored on the mailbox, each performer named as for a creation.
- Refused on a store whose sources declare nothing, without a source performing the capability or with several unchosen ones, and, for a rename, on a name empty or holding a control character. A name holding the server's separator, a sibling of the same name and a mailbox holding others are the performer's to refuse: the store records neither separators nor parents, and the row parks.
- `PimdirCreated` becomes `PimdirQueuedIntent`, the one output of the three backend calls.
- `pimdir queue show` documents where a rename's and a delete's receipt go: the receipt follows a renamed collection to its new id, and leaves the store with a deleted one, the row then reading `unknown`.
- The io-pimdir pin moves to 908db2b, which also gates `$Important` and `$Junk` on `mail.flags.important` and `mail.flags.junk` inside its gate; Himalaya's flag staging names no capability itself, so nothing changes in its code.
- The shared API gains no `mailbox rename` or `delete` verb: what the commands print is the queue row, a pimdir detail. The capability-shared-api task for the protocol-only verbs stays open.

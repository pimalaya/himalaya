---
cairn: change
change: pimdir-cache-backend
---

# Delta

## ADDED Requirements

### Requirement: pimdir cache backend
Himalaya SHALL support a `pimdir` storage backend: a local
[pimdir](https://github.com/pimalaya/pimdir) store (io-pimdir over io-replica) used
as an offline **cache** the sync engine populates, selected local-before-network
like the file backends. It adapts io-pimdir's client read API and the io-replica
`mutate` seam rather than a network client.

Reads SHALL be source-independent and built from the stored `v: 1` meta (pimdir
SPEC §13) without reading bodies: `list_mailboxes` (collections of kind
`message/rfc822`), `list_envelopes` and `search_envelopes` (in-memory sort/filter/
paginate; body search only on hydrated items), and `get_message` (blob read).

### Requirement: Availability-aware reads
`get_message` on an item whose body is not local (`level < Full`, no `object`)
SHALL report a clear "body not fetched" state (the cue to sync), not a data-loss
error; such an item still lists.

### Requirement: pimdir writes are staged mutations
Writes SHALL go through the io-replica `mutate` seam (never raw SQL), so the next
sync derives and pushes them: `store_flags`→`SetFlags`, `add_message`→`Add`
(content hash matching Neverest so an added message dedups against a synced one),
`copy_messages`→`Copy`, `move_messages`→`Move`, `delete_messages`→`Remove`. A write
is attributed to the configured `pimdir.source`; on a store not synced as that
source (the item has no binding) it SHALL fail loudly rather than stage a change no
sync will carry. pimdir has no native trash.

## MODIFIED Requirements

### Requirement: Local storage backends
Maildir, m2dir and pimdir SHALL adapt io-maildir, io-m2dir and io-pimdir. Maildir
stores added messages under `cur/`. m2dir is content-addressed with no native copy
or move, so those are a get plus a store (plus a delete for move), and its flags
live in a `.meta/<id>.flags` sidecar; m2dir mailbox `rename` and message
`copy`/`move` remain unavailable until io-m2dir supports them. pimdir is an offline
cache: reads project the store's shared items and are availability-aware, and writes
are staged io-replica mutations a later sync propagates.

## REMOVED Requirements

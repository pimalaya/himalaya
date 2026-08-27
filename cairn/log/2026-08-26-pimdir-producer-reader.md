---
cairn: log
change: pimdir-producer-reader
landed: 2026-08-26
---

# Himalaya reads the sync store as a reader, and stages its edits as a producer

Two defects against a real Neverest store, one visible from the first command and
one that had not fired yet.

**Mailboxes were named by their store key.** Neverest keys a hub collection
`<namespace>/<name>`, so `INBOX` is stored as `imap/INBOX`. The backend passed the
id through as the name and expected it back on input, so `mailbox list` printed
the id in both columns and `-m INBOX` looked up a collection nothing was written
under: an empty envelope table, no error. `mailbox.alias.inbox` and the implicit
INBOX default were dead for every pimdir account, both resolving to a bare name.

The namespace is now derived (every mail collection of the account sharing one
prefix, which a single-source account always does), `pimdir.namespace` overrides
it, and `hub_id` resolves a typed name against the account's collections, still
taking a full id as itself. A name matching none or several is refused naming the
candidates rather than read as an empty mailbox.

**Writes ran the owner's write path on io-pimdir 0.2.** The five write verbs drove
an io-replica `mutate` coroutine into `ReplicaStorage::write`, which in 0.2 ends
every batch with `collect_garbage`: `DELETE FROM objects WHERE refcount = 0` and
unlink. Neverest's phase 2 streams bodies into the blob tree at refcount 0 and
attaches them in phase 3, which pimdir SPEC §14 invites, so one `himalaya flag
add` during a sync destroys every not-yet-attached body, bytes included. 0.2 takes
no owner lock, so nothing made the two exclusive. The store is gigabytes and the
sync is what refills it.

Himalaya was holding a role it should never have had. It is a reader and a
producer now, which is what the format already provides for: reads open through
`open_read_only` (`_lock: None`, so no lock at all), and each write is one
`PimdirProducer::enqueue` of a `PimdirAction` addressed by the public `seq`, for
the owner to drain. An added body is written to the blob tree and committed
durably first, the queue row pinning it. No index write, no collection load, no
sweep.

**Deps**: io-pimdir 0.2 to 0.3, io-replica 0.3 to 0.4, both patched to git the way
Neverest patches them until they publish. `ReplicaFlags` became an enum there, so
an unknown set now renders as no flags rather than through a tuple field, and a
flag op on one stages a *known* set: carrying the unknown forward would have
erased the markers the sync knows.

**Deleted**: `derive_link_and_meta` and `pimdir/hash.rs`, for
`io_pimdir::conventions`, the one implementation of SPEC Annex A. `pimdir.source`
is gone with the mutate path, replaced by `pimdir.account`: a producer attributes
nothing to a source, and what a reader needs is which account's collections to
show.

**Left standing, deliberately**: `conventions::derive` returns the bare
`Message-ID` as a link id and Neverest writes `mid:<id>`. Every synced store is on
`mid:`, verified on this one (`item list --json` shows
`"link_id":"mid:1.0.C.0.1DD2C438E0EFF2E.0@mail29243.apostello.io"`), so the bare
form would link a message twice and store its body twice: the exact failure
`conventions` was written to end, still live because Neverest never adopted it.
The backend translates at the seam with a NOTE naming the deletion condition.
Which crate moves is theirs to settle.

Verified against the account's own store, 8824 items and 2.3 GiB of blobs:
`mailbox list` names all sixteen mailboxes bare, `-m INBOX` and the alias both
list, an unknown mailbox names the sixteen, `message read` renders from the blob.
Writes were driven against a scratch copy of the index: `flag add` staged
`set-flags seq 5005 -> [\Flagged \Seen]` (the existing marker kept), `message move`
and `message delete` staged `move seq N -> imap/Trash` (the namespace back on the
target), and `message save` wrote the blob to its sharded path and enqueued
`add link mid:staged@himalaya, object aw64hcc…`. 116 tests green, fmt and clippy
clean.

Spec updated: `backends` (ADDED "pimdir names a mailbox the way its server does";
MODIFIED "pimdir is an availability-aware cache" into "pimdir is a reader and a
producer, never the owner", plus "pimdir reads one account" and the public-id
requirement; REMOVED "pimdir writes auto-source").

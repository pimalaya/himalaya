---
cairn: change
id: pimdir-producer-reader
status: landed
created: 2026-08-26
---

# Himalaya read the sync engine's store as its owner, and named its mailboxes wrong

## Why

Two defects, one visible and one not.

**Mailboxes were named by their store key.** Neverest keys a hub collection `<namespace>/<name>`, so the mailbox the server calls `INBOX` is stored as `imap/INBOX`. The backend passed that id through as the display name and expected it back on input, so `mailbox list` printed the id twice and `-m INBOX` looked up a collection nothing was ever written under: an empty envelope table, silently, with no error. `mailbox.alias.inbox` and the implicit-INBOX default were dead for every pimdir account, since both resolve to a bare name.

**Writes ran the owner's write path on io-pimdir 0.2.** `store_flags` and the four other writes drove an io-replica `mutate` coroutine and called `ReplicaStorage::write`, which in 0.2 ends every batch with `collect_garbage`: `DELETE FROM objects WHERE refcount = 0` plus unlinking those blobs. Neverest's hydration phase streams bodies into the blob tree at refcount 0 and attaches them in a later phase, which pimdir SPEC §14 invites, so one `himalaya flag add` during a sync destroys every not-yet-attached body, bytes included. io-pimdir 0.2 takes no owner lock either, so nothing made the two processes exclusive. The store this reads is measured in gigabytes and the sync is what refills it.

Himalaya is not the store's owner and should never have been holding that role: it reads a replica and stages intents. The format already says so — readers take no lock, producers take a shared one and enqueue actions the owner drains.

## What

- io-pimdir 0.2 to 0.3 and io-replica 0.3 to 0.4, both patched to git as Neverest patches them, until they are published.
- Reads open the store with `open_read_only`, which takes no lock, so Himalaya never waits on nor refuses a sync in flight.
- Writes go through `PimdirProducer`: one `enqueue` per operation, addressed by the public `seq`, with the body written to the blob tree and pinned by the queue row before the action referencing it lands. The owner applies them on its next run. No index write, no collection load, no sweep.
- A mailbox is named the way its server names it. The namespace is derived when every mail collection of the account shares one prefix; `pimdir.namespace` overrides. A name is resolved against the account's mail collections, a full id still taken as itself, and a name matching none or several is refused naming the candidates instead of reading as empty.
- `pimdir.source` is replaced by `pimdir.account`: a producer attributes nothing to a source, and what a reader needs is the account whose collections it shows (pimdir SPEC §9.2).
- `derive_link_and_meta` and `pimdir/hash.rs` are deleted for `io_pimdir::conventions`, the one implementation of SPEC Annex A.

## Known divergence, not fixed here

`conventions::derive` returns the bare `Message-ID` as a link id; Neverest writes `mid:<id>`, and every store Neverest has synced is on that spelling. Staging the bare form against one of those would link a message twice and store its body twice, which is the exact failure `conventions` exists to prevent. This backend translates to `mid:` at the seam, with a NOTE naming the deletion condition. Which of the two crates changes is their decision, not Himalaya's.

---
cairn: change
id: duplicate-link-id-mints-an-item
status: landed
created: 2026-08-28
---

# A mailbox may hold one Message-ID twice, and the store now says so

> Cross-repo change, same id in eight repositories. This crate is at the end of the chain, and its part is an audit plus a bump: **pimdir** → **io-replica** → **io-pimdir** → **io-webdav** → **neverest** → **himalaya** (here), **cardamum**, **calendula**.

## Why

Until now a pimdir store held one item per `(collection, link_id)`, and a mailbox holding one `Message-ID` twice (a double delivery, a retried `APPEND`, a restore, a copy of a sent message) resolved to a single frozen item: one of the two was stored, the other was recorded as a handle on the binding and mirrored nowhere. The pimdir backend inherited that as an invariant it never had to state, since the store could not hand it two rows sharing an identity.

That changes. pimdir SPEC §9 makes `link_id` the store's key rather than a restatement of the message's `Message-ID`: the bare id when it is free in the collection, a minted `dup:<hint>#<handle>` when the same source already binds it under another handle. The store now holds both messages, and the backend will list both.

For this crate the change is mostly good news arriving unannounced: an `INBOX` that used to show one of two duplicated messages shows both, which is what the mailbox holds. The work is making sure nothing here assumes what the store no longer guarantees.

## What

- **Nothing may treat a body's `Message-ID` as an address.** A read resolving a `Message-ID` to the item, or re-deriving one to look a row up, is now wrong: the key stays unique, but it is no longer what the body says. The backend already addresses by the public `seq`, so the exposure is limited to whatever resolves an identity directly.
- **`add_message` is unchanged, deliberately.** It stages an `add` carrying the derived link id, and a staged add colliding with a stored identity still parks (pimdir SPEC §15.3): minting is what reading a server requires, parking is what authoring a message locally requires. A local compose that collides is a producer error, not a duplicate to be minted.
- **The link id derivation is unchanged.** `io_pimdir::conventions` returns the bare `Message-ID`, which is what this crate stages, and the `mid:` divergence is already gone (verified 2026-08-28: a real store holds bare ids and no prefixed ones). What changes is only that the store may key an item it received from a source differently from what a derivation would produce.
- **The envelope of a minted item is an ordinary envelope.** No prefix is shown, no badge, no dedup on display. Two messages is what the mailbox holds, and the id the user sees is the `seq`, which differs between the two.

## Scope / non-goals

- **No dedup, no merge, no repair.** A duplicated mailbox is the server's state and the user's to resolve.
- **No new listing behaviour.** Two rows list as two rows, in the order the store pages them.
- **No queue change.** Parking on a colliding local add is the existing rule and stays.

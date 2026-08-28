---
cairn: log
change: duplicate-link-id-mints-an-item
date: 2026-08-28
---

# A mailbox may hold one Message-ID twice, and the listing now shows both

Last of eight repositories on one change id, and the only one whose part is mostly an audit. pimdir SPEC §9 makes `link_id` the store's key rather than a restatement of what the body says: the bare hint while it is free in the collection, a minted `dup:<hint>#<handle>` when the same source already binds it under another handle. A mailbox holding one `Message-ID` twice, which a double delivery, a retried append, a restore or a copy of a sent message all produce, used to resolve to one item, the loser recorded on the winner's binding and mirrored nowhere. It is now two items, so Himalaya lists two messages where it listed one, and the message that was invisible can be read. Capability `backends` moved.

## What landed

- **The audit found nothing to fix, which is the finding.** Every read addresses by the public `seq` (`parse_id` then `get_item`), the keyset cursor in `scan_items` pages on `link_id`, which stays unique per collection, and `Envelope.message_id` is display-only with nothing keying on it. Nothing here ever read `bindings.ambiguous_handles` or `ReplicaStatus::Ambiguous`, so io-pimdir dropping both is not breaking. The backend addressing by `seq` since `pimdir-public-id` is what paid for that: an address derived from the body would now be ambiguous where a `seq` is not.

- **A `Message-ID` is not an address** (spec, added). The rule the backend has been following silently is written down, since it is now load-bearing rather than incidental: `(collection, link_id)` still names one item and `seq` still names one message, but the link id is no longer derivable from the body, so a read that re-derives an identity to address a row is addressing an unknown number of them.

- **The `mid:` era's last two sentences go.** `add_message`'s doc and the rationale on its test both said an append "deduplicates against a synced copy rather than linking one message twice", and the spec's producer requirement said the backend spells the link id "the way the store it writes to already does, so an added message deduplicates against a synced copy". Neither half was true: a staged add whose link id the collection already holds parks (pimdir SPEC §15.3), and the store no longer spells the key the way a derivation would. All three now say what happens and why the two answers differ. Minting is the store's answer to what a source hands over, a replica owing the collection what the collection holds; parking is its answer to a producer authoring a message the collection already has, which named a key it does not own and is told so rather than having its message filed under one it never asked for.

- **No display change.** No prefix is shown, no badge, no dedup on the listing. Two messages is what the mailbox holds, and the ids the user sees are the two `seq`s. Whether the pair is worth removing is the server's state and the user's call.

- **`add_message` itself is untouched.** It derives the bare `Message-ID` through `io_pimdir::conventions` and stages it, which is exactly what a producer owes the store.

## Dependencies

`[patch.crates-io]` now points io-pimdir and io-replica at the sibling checkouts rather than at git revisions that predate the minted key. Neither crate is released, and the whole local suite is being made testable end to end; the entries go back to git or to the published versions when they are. Cargo.lock loses the two `source` lines accordingly.

## Tests

src/pimdir/backend.rs: `two_items_sharing_a_message_id_project_two_public_ids`, two items of one mailbox whose meta carries one `Message-ID`, the second under a minted `dup:twice@host#1174`, projecting two envelopes that share the displayed `Message-ID`, carry their own `seq` as the id, and show no trace of the minted key. The repo has no store fixture and every pimdir test is a unit test over a hand-built item, so that is the whole of what is testable here; parking is the store owner's behaviour and is covered in io-pimdir's `tests/duplicate_link_id.rs`.

119 tests pass, `cargo clippy --all-targets` is clean but for one warning in src/wizard/search.rs that predates this change, `cargo fmt`.

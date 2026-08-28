---
cairn: tasks
change: duplicate-link-id-mints-an-item
---

# Tasks

- [x] Bump io-pimdir (and io-replica through it) to the releases carrying the minted key and no ambiguity surface. Neither is released yet, so `[patch.crates-io]` points both at the sibling checkouts (`path = "../io-pimdir"`, `path = "../io-replica"`) instead of the stale git revisions it named, which resolve to no minted key at all. The whole local suite is testable end to end that way.
- [ ] Point the two `[patch.crates-io]` entries back at git, or drop them for the published `version` requirements, once io-pimdir and io-replica are released. A path patch is a working-tree convenience and must not ship in a release.
- [x] Audit src/pimdir/ for a read that assumes one item per identity: none found. Every read addresses by `seq` (`parse_id` then `get_item`), the keyset cursor in `scan_items` pages on `link_id`, which stays unique per collection, and `Envelope.message_id` is display-only with no consumer keying on it. Nothing in the repo reads `ambiguous_handles` or `ReplicaStatus::Ambiguous`, so io-pimdir's removal is not breaking here.
- [x] `mid:` translation at the seam: already gone. No occurrence outside cairn/log and one historical proposal, and `add_message` passes the derived bare id straight through.
- [x] `add_message` needs no change: it stages the derived bare `Message-ID` and returns it for display only, and a collision parks in the store rather than minting.
- [x] Fix the stale rationale the `mid:` era left behind, which the audit found still asserting the opposite of the new rule: src/pimdir/backend.rs around the `add_message` doc and its test rationale both say an append "deduplicates against a synced copy rather than linking one message twice". A colliding staged add parks (pimdir SPEC §15.3); it neither deduplicates nor mints. Both now say what happens and why the store answers a source and a producer differently.
- [x] Tests: two items of one mailbox whose bodies carry one `Message-ID` project two envelopes with distinct public ids. `two_items_sharing_a_message_id_project_two_public_ids` in src/pimdir/backend.rs, over two hand-built items, the second under a minted `dup:` key. Nothing beyond that is testable here, the repo having no store fixture and every pimdir test being a unit test over a hand-built item; the parking behaviour is the store owner's and is tested in io-pimdir.
- [x] `cargo test` (119 passed), `cargo clippy --all-targets` (one pre-existing warning in src/wizard/search.rs, untouched), `cargo fmt`.
- [x] CHANGELOG `### Fixed`: a duplicated message no longer disappears from the pimdir backend's listing.
- [x] Fold `delta.md` into `cairn/spec/backends.md`. Two requirements move, not one: `pimdir shows a short public id`, and `pimdir is a reader and a producer, never the owner`, whose last paragraph is the surviving `mid:`-era sentence ("SHALL spell the link id the way the store it writes to already does, so an added message deduplicates against a synced copy"). Both halves of it are now wrong. The added requirement heads the pimdir cluster; its scenario is prose, no spec file here carrying a scenario block.
- [x] Append the log entry; mark `landed`.

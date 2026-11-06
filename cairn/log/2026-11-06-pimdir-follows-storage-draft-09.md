---
cairn: log
change: pimdir-follows-storage-draft-09
landed: 2026-11-06
---

# The pimdir backend follows storage-draft-09 and search-draft-09

Pinned to io-pimdir 5c18a02 (0.9.0: pimdir storage-draft-09, sync-draft-08, search-draft-09) by git rev, the requirement `0.9`.

- Build: no compile change. `PimdirSearchClock` and `PimdirSearchOptions` losing `Copy` touched nothing, Himalaya passing both by reference; `PimdirLoadSelection::Owing` and the flag bits are the sync engine's and the store's. Cargo.lock: io-pimdir 0.9.0, ical-rs 0.6.1 (git 99b9273) to 0.7.0, vcard-rs 0.5.2 to 0.6.0, mime-rs git d7ea032 to 0.1.0, and with it base64-codec, quoted-printable-codec and simd-cpu from git 0.0.1 to crates.io 0.1.0; hashlink added (io-pimdir's statement cache).
- mime-rs: Himalaya's own dependency moves from git d7ea032 to `0.1` from crates.io, the one io-pimdir takes. No public item changed between them (the 0.1.0 diff touches no `pub` signature), and the whole suite passes on it, so the graph holds one mime-rs. The ical-rs patch to 99b9273 is dropped: io-pimdir 0.9 takes ical-rs 0.7 from crates.io and Himalaya does not depend on it.
- `store-version`: a draft-08 store is refused, never migrated. `pimdir_error` already says to upgrade Himalaya when the store's sync engine is newer, else to reinstall the store as neverest's MIGRATION.md says, and names no draft, so the message stands. The tests stamping another draft move to the draft just left and the next one: `a_store_of_another_draft_is_refused_saying_to_reinstall_it` (storage-draft-08 and storage-draft-10), `a_cancel_leaves_a_store_of_another_draft_alone` (storage-draft-08), `an_index_of_another_draft_is_refused_as_index_version` (search-draft-08). They pass as they did, the refusal being io-pimdir's.

Verified: `cargo test --all-features` green (393 unit tests, 1 ignored, and the 44 of tests/), `cargo clippy --all-features --all-targets` clean. Each backend checked alone with rustls-ring (`--all-targets`), `--features pimdir` alone and with `vendored` built, the CONTRIBUTING.md set built and tested: no error, and no warning in src/pimdir. The msgraph-, sieve- and wizard-alone and featureless builds keep the dead-code warnings master has (19, 85, 101 and 91 on master, 86 for the featureless one now).

Spec: search (pimdir searches through the store's index: search-draft-09 §8; the scenario An index of an earlier draft: search-draft-08).

Left: the end-to-end run on a scratch store waits for neverest on io-pimdir 5c18a02. neverest's MIGRATION.md still names storage-draft-08 as the current draft; until neverest ships draft-09, a user moving Himalaya first has a store this Himalaya refuses and no neverest to reinstall it with.

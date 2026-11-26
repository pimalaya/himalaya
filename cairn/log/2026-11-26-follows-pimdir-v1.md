---
cairn: log
change: follows-pimdir-v1
landed: 2026-11-26
---

# Himalaya follows pimdir v1

The pimdir backend moved from io-pimdir 0.20.4 (storage-draft-17, sync-draft-15, search-draft-17) to io-pimdir 1.0.0, which follows pimdir storage-v1.0.0, sync-v1.0.0 and search-v1.0.0. The dependency reads `version = "1.0.0"`, from crates.io.

A store any 0.x release wrote, every store Himalaya 3.0.0 read among them, is refused `store-version` and its index `index-version`, never migrated: the user syncs it with the neverest that wrote it until nothing is queued, moves its directory aside, and lets a v1 neverest sync a new one. There is no migration from 0.x to v1.

## What landed

- Cargo.toml and Cargo.lock: io-pimdir `1.0.0` in the dependencies and the dev-dependencies, features unchanged.
- src/pimdir/search.rs and src/pimdir/index.rs: `PimdirSearchCursor` and `PimdirSearchSort` imported from `io_pimdir::search::plan`, where 1.0.0 moved them from `client::search`.
- Tests: `PimdirStore::for_source` returns a `Result` in 1.0.0, unwrapped at every test site (src/pimdir/backend.rs, src/pimdir/index.rs, tests/pimdir_queue.rs, tests/pimdir_search.rs, tests/pimdir_refusals.rs, tests/pimdir_attachments.rs).
- src/pimdir/backend.rs: `a_store_of_another_version_is_refused_saying_which_is_earlier` (renamed from `a_store_of_another_draft_...`) stamps storage-draft-11, -17 and -99 as earlier stores, every draft now ranking below `storage-v1`, and a `storage-v2` store reading takes version 2 to as the later one asking to upgrade himalaya; `a_later_store_himalaya_still_reads_opens` pins that a `storage-v2` store whose `read_version` is 1 opens (pimdir STORAGE §4.2), which io-pimdir 0.20.4 refused.
- tests/pimdir_queue.rs: `a_copy_into_a_mailbox_holding_the_message_lists_one_envelope` gives the imap source a checkpoint on `imap/Archive`. Under sync-v1 §7 a copy's create is staged under the run's source only where that source syncs the target (a binding or a checkpoint there), else under the source an `add` there runs as; the fixture's imap source never synced `imap/Archive`, so the create went to the jmap source, which holds the message live there, and was minted as a second message, as §7 states. With the checkpoint the create stays imap's and joins the jmap member, the case the test pins.
- Doc comments naming a draft (`ErrorCode::IndexVersion`, `PimdirClient::cancel_queued`) speak of versions.
- CHANGELOG.md [Unreleased] and MIGRATION.md say a store an earlier release wrote is refused and how to start over.

## Capabilities moved

- [backends](../spec/backends.md): a `move` into a mailbox no source syncs folds into nothing in the overlay, the owner parking it; Himalaya reads storage-v1; a draft's store, every 0.x store among them, is `store-version`; a later release's store is read while its `read_version` is at most 1 and queued into while its `produce_version` is too, judged by `store_meta` alone; `queue cancel` opens schema version 1 alone.
- [search](../spec/search.md): `pimdir search` answers search-v1 §8; an index of any search draft is `index-version` until the sync engine rebuilds it.
- [commands](../spec/commands.md): the `store-version`, `store-corrupt` and `index-version` rows speak of versions, not drafts.

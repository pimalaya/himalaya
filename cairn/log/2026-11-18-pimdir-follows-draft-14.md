---
cairn: log
change: pimdir-follows-draft-14
landed: 2026-11-18
---

# Pimdir follows draft-14

Lines C1 and C4 of pimdir's audit-2026-11-12, closed for io-pimdir in 0.17.0 (released 2026-11-18), the command-line clients following.

## What landed

- Himalaya moved onto pimalaya-cli 0.6.1, pimalaya-config 0.6.0, io-jmap 0.10.1 and io-pimdir 0.17.0 (storage-draft-14, sync-draft-14, search-draft-14), with no [patch]. A storage-draft-13 store is now refused `store-version`, never migrated.
- C1: a `store-version` refusal reads io-pimdir's `PimdirFormatRefusal` naming himalaya, which says which store is the earlier and what that asks, the wording Cardamum and Calendula show too. The local `reinstall` text is gone.
- C4: a pimdir write prints its row as `PimdirActionHandle` (`"row":"<store id>.<incarnation>.<id>"`), and `pimdir queue show` and `pimdir queue cancel` take that handle, checked against the store, or a bare id as `queue list` prints it. A handle of a restored copy reads `reset` in `queue show` and is refused `store-reset` by `queue cancel`, before any prompt, the row left queued.
- The local `private_options` of the attachment download is gone, pimalaya-cli's `file::private_options` taking its place there and in the mbox cache.

## Capabilities moved

backends (store-version wording, the handle requirement, receipt shapes), commands (`store-version` row), search (search-draft-14).

## Verified

`cargo test --all-features`: 756 unit tests and every integration suite pass; `cargo clippy --all-features --all-targets` clean.

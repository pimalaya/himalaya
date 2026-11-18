---
cairn: change
id: pimdir-follows-draft-14
status: landed
created: 2026-11-18
---

# Pimdir follows draft-14

## Why

io-pimdir 0.17.0, released 2026-11-18, follows pimdir storage-draft-14, sync-draft-14 and search-draft-14, and closes two lines of pimdir's audit-2026-11-12 for the crate, the command-line clients following:

- C1: the refusal of a store of another format version had one wording per program. STORAGE §4.2 now orders format versions, and `version::PimdirFormatRefusal` is the one wording, saying which store is the earlier and what that asks.
- C4: a queue row was followed and cancelled by a bare integer, without the store id and incarnation STORAGE §4.5 asks a producer to record. `PimdirActionHandle` names the row under the store's identity, and `action_status` and `drop_action` check it (`Reset`, `store-reset`).

pimalaya-cli 0.6.1 ships `file::private_options`, which himalaya carried a local copy of, and pimalaya-config 0.6.0 and io-jmap 0.10.1 are released beside them.

## What

- Move onto pimalaya-cli 0.6.1, pimalaya-config 0.6.0, io-jmap 0.10.1 and io-pimdir 0.17.0, no [patch].
- Drop the local `private_options` of the attachment download, and open the mbox cache files through the same helper.
- Word a `store-version` refusal through `PimdirFormatRefusal`, naming himalaya.
- Print a pimdir write's `row` and `copyRow` as the row's handle, and take a handle or a bare id in `pimdir queue show` and `pimdir queue cancel`, a handle of a restored copy refused `store-reset`.

## Not in scope

The other audit lines (C3 attachment names under §18, C5 hoisting the store rules into io-pimdir). `pimdir queue list` keeps the bare id, read from the store as it stands.

---
cairn: change
id: pimdir-follows-storage-draft-08
status: landed
created: 2026-11-01
---

# The pimdir backend follows storage-draft-08 and search-draft-08

## Why

io-pimdir bff5737 (pushed, master, still 0.8.0) follows pimdir d671bbd: storage-draft-08, sync-draft-07, search-draft-08 (io-pimdir cairn/changes/the-crate-follows-storage-draft-08). himalaya reads and queues behind neverest, and builds the search index (cairn/changes/pimdir-search).

Built on 2026-11-01 against ../io-pimdir at bff5737 (`cargo check --all-targets`, default features, over the uncommitted work of 2026-10-30):

- Resolution fails first: io-pimdir takes rusqlite 0.41, himalaya pins `rusqlite = "0.40"` with `bundled` (Cargo.toml:42).
- With rusqlite at 0.41, 16 errors:
  - `PimdirBlobs::path` answers `io::Result<PathBuf>` (src/pimdir/backend.rs:643, 644, 651, 1838, 1839, 1915, 1916, and the `exists` calls).
  - `intent::PimdirSubmit` carries no `object` (the message is the row's body, `to_action(message)`) and `source` is a `String` (src/pimdir/backend.rs:2051 to 2117, 3468).
  - `hash_algo()` and `hash::PimdirHashAlgo` are gone, `PimdirBlobs::open` takes the directory alone (src/pimdir/client.rs:74, tests/pimdir_attachments.rs:22).
  - The tag types are at `io_pimdir::search::tag`, `client::search::tag` being private (src/pimdir/index.rs:28).
- A second pass will show what type errors hide: `#[non_exhaustive]` `PimdirError` and `PimdirQueryErrorCode` (`limit-exceeded` is new), `PimdirReader::address_placements` paged, `count_mail_by_day`'s `shift` in minutes, `PimdirDrainReport` and `PimdirGcReport` no longer `Copy`.

## What the compiler does not see

- **Error codes**: io-pimdir no longer raises `store-stale` or `store-newer`; it adds `store-version`, `sync-version`, `store-exposed`, `index-version`, `index-zone-unknown`, `flag-invalid`, `action-malformed`, `conflict-unread` (src/error.rs, src/json_schema.rs). A store an earlier neverest wrote is refused as `store-version`: the message points at neverest's reinstall (its MIGRATION.md), never at deleting the store.
- **Flags**: an empty flag or one holding a space, a control or a non-ASCII character is refused at the enqueue as `flag-invalid`; himalaya's flag verbs report it rather than queue.
- **Search**: the query limits (1,024 scalar values, 32 groups, 128 terms) answer `limit-exceeded` in the ariadne rendering; an index of another draft is `index-version`, rebuilt by the indexer when earlier; `PimdirSearchLocal::behind` and the coverage carry over.
- **Modes**: a store another user can read is refused as `store-exposed`; test fixtures under `tempfile` are `0700` already.

## Pin

Built against `path = "../io-pimdir"` while porting, then `[patch.crates-io]` by git rev bff5737; the requirement stays `0.8` until the user releases io-pimdir.

---
cairn: tasks
change: pimdir-follows-storage-draft-08
---

# Tasks

- [x] The 2026-10-30 work committed first (user, 2026-11-01)
- [x] `rusqlite = "0.41"`; Cargo.lock moves io-pimdir, rusqlite and libsqlite3-sys alone (plus the `windows-sys` edge of errno, rustix, tempfile and winapi-util, no new version)
- [x] The compile port (proposal's list, then the second pass); `cargo test` green on what it asserted before
- [x] Error vocabulary and messages; a test opening a store whose `store_meta.format_version` was rewritten (rusqlite) and reading `store-version`
- [x] `flag-invalid` from `flag add`; `limit-exceeded` rendered; tests failing first (`limit-exceeded` needed no change: covered by a test that passed at once)
- [x] End to end on a scratch store neverest synced from a test account (cardamum and calendula beside): envelope list, read, search, flag, send queued, drained by neverest (2026-11-01, on local Stalwart IMAP and Radicale DAV rather than a provider test account: every queued write reached its server after `neverest sync`)
- [x] Pin to io-pimdir bff5737 by git rev, path patch dropped
- [x] Delta, fold into spec/backends.md, spec/search.md and spec/commands.md, CHANGELOG; log
- [x] Land

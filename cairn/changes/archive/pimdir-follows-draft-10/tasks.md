---
cairn: tasks
change: pimdir-follows-draft-10
---

# Tasks

- [x] `io-pimdir = "0.11"` from crates.io (46e8b4ea)
- [x] The compile port; the tests stamping another draft moved to `storage-draft-09`, `search-draft-09` and `storage-draft-11`
- [x] One gate per enqueue; `stage_flags` as one batch naming the message refused
- [x] `set-flags` states `base` for an add and a remove; a test where a sync lands a flag before the drain, failing with `base` left out
- [x] `remove` with no `if_match` and no `resolves`
- [x] Queue states `unconfirmed` and `reset`, an applied row with no collection, the unconfirmed mark in `queue list`, the wording of `queue cancel`
- [x] Coverage: `missing`, `unparseable`, `encrypted`, said only while a term reads them
- [x] `store-reset` and `lock-replaced` relayed
- [x] Delta, fold into spec/backends.md, spec/search.md and spec/commands.md, CHANGELOG, MIGRATION; log
- [ ] The suites of tests/ and an end-to-end run on a scratch store a draft-10 neverest synced (after every app moved, owner 2026-11-13)

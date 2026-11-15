---
cairn: tasks
change: a-staged-body-is-written-under-the-lock
---

- [x] Cargo.toml: io-pimdir pinned to 4c3353e, `cargo update -p io-pimdir` only
- [x] `supersedes: None` on the `Add` himalaya builds (src/pimdir/backend.rs, and the test fixture)
- [x] Test first, failing on the old order: with `objects.lock` held exclusively, a staged add writes no body; released, it lands and is queued
- [x] `add_message` and `send_message` open the producer first and write through `producer.blobs()`
- [x] Fold into spec/backends.md, log, CHANGELOG

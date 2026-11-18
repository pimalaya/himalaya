---
cairn: tasks
change: the-mbox-cache-is-private
---

# Tasks

- [x] Test `a_cache_is_private_to_its_owner` (src/mbox/cache.rs): a directory left at 0755 stayed 0755 on the old code; now 0700, the file 0600.
- [x] `create_private_dir` and `write_private`.
- [x] Fold into cairn/spec/backends.md; CHANGELOG; the log entry.

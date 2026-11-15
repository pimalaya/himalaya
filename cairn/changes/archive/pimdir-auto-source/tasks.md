---
cairn: tasks
change: pimdir-auto-source
---

# Tasks

- [x] `pimdir/client.rs`: when `pimdir.source` is unset, probe
      `distinct_sources()` and write as the single source, else `local`.
- [x] Build/test/fmt; verify write-back with no `pimdir.source` set.
- [x] Fold into `cairn/spec/backends.md`; log; land.

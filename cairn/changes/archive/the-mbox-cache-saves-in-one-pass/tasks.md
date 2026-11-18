---
cairn: tasks
change: the-mbox-cache-saves-in-one-pass
---

# Tasks

- [x] src/mbox/cache.rs: `save` over a `HashSet`.
- [x] No regression test: the change is one of complexity alone, and timing runs are out; the mbox suites pass.
- [x] Fold into cairn/spec/backends.md; CHANGELOG.

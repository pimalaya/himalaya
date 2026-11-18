---
cairn: tasks
change: local-listings-hold-a-few-messages
---

# Tasks

- [x] src/maildir/backend.rs: `visit_entries`, `READ_CHUNK`.
- [x] src/mbox/backend.rs: `envelopes` takes the filter as `keep`.
- [x] No regression test: the change bounds memory alone, which no test measures; the listing and search suites pass unchanged.
- [x] Fold into cairn/spec/backends.md; CHANGELOG.

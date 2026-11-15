---
cairn: tasks
change: a-utc-instant-ends-with-z
---

# Tasks

- [x] `rfc3339` in src/shared/message/read.rs; `internalDate` in src/imap/fetch.rs.
- [x] `date_keeps_its_offset_utc_as_z_and_an_unreadable_one_is_null` (it read `+00:00` before); tests/local_stores.rs reads `Z` through the binary.
- [x] Fold into cairn/spec/commands.md; CHANGELOG and MIGRATION.md; the log entry.

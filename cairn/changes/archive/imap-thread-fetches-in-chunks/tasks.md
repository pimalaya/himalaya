---
cairn: tasks
change: imap-thread-fetches-in-chunks
---

# Tasks

- [x] src/imap/envelope/thread.rs: `FETCH_CHUNK`.
- [x] Test `a_long_thread_listing_fetches_in_chunks` (600 scattered ids against a loopback server), failing on e440b4e4 (one `FETCH`).
- [x] Fold into cairn/spec/backends.md; CHANGELOG.

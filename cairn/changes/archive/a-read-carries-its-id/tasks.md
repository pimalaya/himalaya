---
cairn: tasks
change: a-read-carries-its-id
---

# Tasks

- [x] `MessageReadOutput { id, etag, view }`, the view renamed `MessageReadView` for `message parse`.
- [x] tests/local_stores.rs (Maildir and mbox), tests/repl.rs and tests/mcp.rs read `id` and `etag`; tests/message_parse.rs compares the parse with the read less `id` and `etag`.
- [x] Fold into cairn/spec/commands.md; the CHANGELOG line and the MIGRATION.md bullet; the log entry.

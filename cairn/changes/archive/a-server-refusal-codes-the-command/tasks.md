---
cairn: tasks
change: a-server-refusal-codes-the-command
---

# Tasks

- [x] `error::code`, the readers and `coded` over them; `coded` on each REPL line, `code` on each `account check` row.
- [x] The IMAP, Gmail, Graph and JMAP readers; the Gmail and Graph item errors on the same mapping.
- [x] `jmap::error::set_error` at the whole-command JMAP writes.
- [x] `message-not-found` on an unknown id read on every backend.
- [x] Tests: the readers (unit), a scripted IMAP server refusing `CREATE` through a one-shot run, a REPL line and an MCP call (tests/refusal.rs, failing without the readers and, for the REPL, without its `coded`), an unknown id read on Maildir, mbox, pimdir and JMAP, in a REPL and over MCP.
- [x] Fold into cairn/spec/commands.md and cairn/spec/repl.md; the CHANGELOG lines; the log entry.

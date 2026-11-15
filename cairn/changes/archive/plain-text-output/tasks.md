---
cairn: tasks
change: plain-text-output
---

# Tasks

- [x] Lists on `Columns`, free-text column last: shared mailbox, envelope (list, search), attachment (list, download); account list and capability; Gmail drafts, labels, messages, threads, history and settings lists; IMAP flag, mailbox list, search, sort; JMAP email, identity, mailbox, submission, thread; Maildir, m2dir and mbox lists and flag lists; Graph attachments, mail folders, messages; pimdir attachment, flag, mailbox, queue lists and search, thread, tag; sieve list.
- [x] Records on `Fields`: account check, imap select, status, id, fetch, shared and JMAP vacation, sieve capability, Gmail and Graph gets, Gmail threads get, message read header block.
- [x] Newline gaps: mbox, maildir, m2dir message save, Gmail draft writes, jmap query, sieve get (only when missing), jmap email parse (bodies apart).
- [x] Drop the blank line before lists and the `Found N message(s)` trailer of `imap search` and `imap sort`.
- [x] Config: drop the colour keys, `table.preset`, `table.arrangement` and the structs left empty; keep the glyph keys; regression test that a colour key is refused.
- [x] Drop `-w/--max-width` from the 8 commands and the MCP filter hiding it.
- [x] Cargo.toml: crossterm out; clap `default-features = false` with derive, std, help, usage, error-context, suggestions, env.
- [x] ariadne without colour in src/shared/envelope/search.rs and src/pimdir/index.rs.
- [x] `table::sanitize` to `sanitize::text` everywhere; src/shared/table.rs deleted.
- [x] Tests asserting the old output moved to the new one; a list test pinning the exact columns.
- [x] README, config.sample.toml, CHANGELOG.
- [x] Fold the delta into the spec, log, set status landed.

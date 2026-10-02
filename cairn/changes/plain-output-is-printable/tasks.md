---
cairn: tasks
change: plain-output-is-printable
---

# Tasks

- [x] `printable` in src/shared/table.rs, with unit tests.
- [x] `envelope list` and `envelope search`: id, subject, and the FROM or TO names.
- [x] `attachment list`: id, filename, type and path.
- [ ] Review and convert the other renderings of server-supplied strings: src/imap/fetch.rs, src/imap/mailbox/list.rs, src/imap/id.rs, src/pimdir/queue/list.rs, src/jmap/identity/delete.rs, src/jmap/vacation/get.rs, src/gmail/threads/get.rs, src/gmail/threads/list.rs, src/gmail/settings/sendas/get.rs, src/gmail/settings/vacation/get.rs, src/gmail/settings/filters/summary.rs, src/msgraph/attachments/list.rs, src/msgraph/mail_folders/list.rs.
- [ ] Check the binary-content test in src/shared/output.rs, which lets DEL and C1 bytes through to the terminal.
- [ ] Fold the delta into [cairn/spec/commands.md](../../spec/commands.md); write the log entry.

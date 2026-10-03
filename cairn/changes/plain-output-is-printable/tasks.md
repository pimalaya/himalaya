---
cairn: tasks
change: plain-output-is-printable
---

# Tasks

- [x] `sanitize` in pimalaya-cli 0.2.6, replacing the local `printable`.
- [x] `envelope list` and `envelope search`: id, subject, and the FROM or TO names.
- [x] `attachment list`: id, filename, type and path.
- [x] IMAP: `imap fetch` headers and structure, `imap mailbox list`, `imap id`.
- [x] `pimdir queue list`: subject and recipients.
- [x] JMAP: set errors (`format_set_error`), `jmap identity delete`, `jmap vacation get`.
- [x] Gmail: `threads get`, `threads list`, `settings sendas get`, `settings vacation get`, filter ids and summaries.
- [x] Graph: `attachments list`, `mail-folders list`.
- [x] The binary check of src/shared/output.rs also refuses DEL and C1.
- [x] Fold the delta into [cairn/spec/commands.md](../../spec/commands.md); write [cairn/log/2026-10-03-plain-output-is-printable.md](../../log/2026-10-03-plain-output-is-printable.md).

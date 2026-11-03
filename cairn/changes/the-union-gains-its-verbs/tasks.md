---
cairn: tasks
change: the-union-gains-its-verbs
---

# Tasks

- [x] Capabilities `mailbox.create`, `mailbox.rename`, `mailbox.delete`, `flags.important`, `flags.junk`, `vacation` in src/capability.rs, every backend's table, m2dir split from Maildir
- [x] Vacation resolves on its own, first serving backend in the storage order; resolver test
- [x] `EmailClient::create_mailbox`, `rename_mailbox`, `delete_mailbox` and the per-backend adapters; cached mailbox indexes dropped after a write
- [x] Shared `mailbox create|rename|delete` with `MailboxCreateOutput`, `MailboxRenameOutput`, `MailboxDeleteOutput`
- [x] `FlagArg::Important` and `FlagArg::Junk`; IMAP `PERMANENTFLAGS` check after `SELECT`
- [x] Shared `vacation get|set` over the JMAP and Gmail clients, `VacationGetOutput`, `VacationSetOutput`
- [x] json-schema entries, MCP tools classified, `--help` footers
- [x] Tests: the resolver, the name joining and splitting, the flag capabilities, the PERMANENTFLAGS decision, the vacation date conversions, the MCP listing
- [x] `cargo build`, `cargo test`, single-backend builds, `cargo fmt`, clippy
- [x] CHANGELOG, MIGRATION (the IMAP keyword refusal)
- [x] Fold delta.md into cairn/spec/commands.md and cairn/spec/backends.md; log entry; status `landed`

## Follow-ups

- [ ] io-jmap: let `JmapVacationResponseUpdate` send `null`, then `vacation set --no-from|--no-to`
- [ ] io-msgraph: `mailboxSettings.automaticRepliesSetting`, then Graph serves `vacation`
- [ ] io-msgraph: child-folder creation, so Graph creates under a parent in one request
- [ ] A shared `mailbox move` (reparenting), `thread` and `identity list`, each its own change

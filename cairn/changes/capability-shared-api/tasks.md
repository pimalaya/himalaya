---
cairn: tasks
change: capability-shared-api
---

# Tasks

- [x] Inventory every operation across backends, named on pimdir Annex B (proposal, "Capability inventory")
- [x] Settle the `?` cells: Gmail and Graph search (`partial`), mailbox reparenting (JMAP, Graph, and IMAP and Gmail by rename), Gmail `DRAFT` (`none`)
- [x] Decide `message.copy` on JMAP and Gmail (one message in two mailboxes): `partial` with a detail (proposal, "Decided while implementing")
- [x] pimdir: propose the new write names as Annex B additions: pimdir cairn/changes/mail-write-names-from-himalaya (draft), `collection.rename`, `collection.delete`, `mail.flags.important`, `mail.flags.junk`; the draft verbs and `submit.cancel` argued out
- [x] Capability enum + per-backend static tables (src/capability.rs, one table per backend, kept in every build), pimdir's grammar without the domain word, levels `full`, `partial`, `server`, `none` with a detail; the pimdir table is `full`, the store's declared rows answering with pimdir's own codes
- [x] `flag add|set|remove` of `\Answered` and `\Draft` on Gmail and Graph: `capability-missing` instead of the silent success (tests/capability.rs, failing on the old code)
- [x] Resolver (`Resolution`): the storage order and the sender in one place, `open_storage` building what it picked, `--backend` as filter; tests in src/capability.rs
- [x] Dynamic tier: JMAP `submit` without `urn:ietf:params:jmap:submission` refused before the upload; IMAP removal without UIDPLUS already reported as flagged; MOVE and SORT have same-result fallbacks; PERMANENTFLAGS decides no shared flag; no shared verb on ManageSieve
- [x] The id-exchanging verbs resolve to one storage transport; test on an account declaring IMAP and Maildir
- [x] `capability-missing` (account, capability, backend or `--backend` filter, detail), in the code table; search options refused as `search-unsupported`
- [x] MCP: `tools/list` filtered by the static tier (an SMTP-only account lists `message_send`, not `envelope_list`; Graph lists no `message_add`); the `envelope_search` description from the tables; tests in tests/mcp.rs, failing on the old code
- [x] The shared tree compiled in every build with a mail transport, dispatch gated; the SMTP-only build compiles again; the shared `json-schema` output, `himalaya-error` included, identical for the SMTP-only and the full build (compared file by file, 19 schemas). Protocol namespaces stay gated with their modules
- [x] `--help` footer per shared leaf generated from the tables. The manual pages are per command group (pimalaya-cli), so the leaf footers reach `--help` only: a pimalaya-cli follow-up
- [x] ~~Compatibility suite against 2.x~~: superseded by the v3 decision, master changing the JSON shapes already; the existing suite (404 tests) holds every served verb's output
- [x] `cargo build`, `cargo test` (404 passed), single-backend builds (imap, maildir, gmail+smtp, jmap, pimdir, mbox, m2dir, msgraph, smtp) without new warnings, `cargo fmt`, clippy clean
- [x] MIGRATION.md: the v2 to v3 section, `capability-missing` among the commands that now fail instead of pretending
- [x] Rewrite the cairn/spec/commands.md opening paragraph
- [x] Fold delta.md into cairn/spec/commands.md, cairn/spec/backends.md and cairn/spec/mcp.md; log entry; status `landed`

## Follow-ups

- [ ] pimalaya-cli: render each leaf's `after_long_help` in the manual pages
- [x] ~~The registry rows of the inventory's protocol-only verbs, each with its shared verb (`mailbox.create`, `mailbox.rename`, `thread`, `vacation`)~~ Done for `mailbox.create`, `mailbox.rename` and `vacation` (the-union-gains-its-verbs, 2026-11-03); `thread` is that change's follow-up.
- [ ] Gmail and Graph file every sent message themselves, so `message send --save` there files a second copy; declare `submit.copy` with that detail and skip the add

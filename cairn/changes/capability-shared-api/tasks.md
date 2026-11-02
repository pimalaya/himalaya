---
cairn: tasks
change: capability-shared-api
---

# Tasks

- [x] Inventory every operation across backends, named on pimdir Annex B (proposal, "Capability inventory")
- [x] Settle the `?` cells: Gmail and Graph search (`partial`), mailbox reparenting (JMAP, Graph, and IMAP and Gmail by rename), Gmail `DRAFT` (`none`)
- [ ] Decide `message.copy` on JMAP and Gmail (one message in two mailboxes): `partial` with a detail, or a name of its own
- [ ] pimdir: propose the new write names as Annex B additions (`collection.rename`, `collection.delete` in pimdir's spelling, `mailbox.*` here, `mail.flags.important`, `mail.flags.junk`, `mail.draft.*`, `mail.submit.cancel`)
- [ ] Capability enum + per-backend static registry (one declaration site per backend, next to its adapter), with pimdir's grammar and `full`/`partial`/`none` plus detail, without the domain word (`message.move`, `mailbox.create`); a pimdir account reads the store's declared rows, `mail.` dropped and `collection.` read as `mailbox.`.
- [ ] `flag add|set|remove` of `\Answered` on Gmail and Graph: `capability-missing` instead of the silent success it reports today
- [ ] Resolver: verb -> capability -> configured transport, with `--backend` as filter and explicit override; assert it reproduces today's `select_storage` order for every existing shared verb.
- [ ] Dynamic tier: per-client capability probe (IMAP `CAPABILITY`, JMAP session, ManageSieve `SIEVE`) consulted only after connection, only for verbs that need it.
- [ ] Resolver: the id-exchanging verbs (mailbox, envelope, message, flag, attachment) resolve to one storage transport; a test on an account declaring two storage backends.
- [ ] Capability error type: account, resolved backend, missing capability, protocol-specific alternative, the code `capability-missing` (added to the code table); non-zero exit, printed like every other error.
- [ ] MCP: `tools/list` lists the verbs the static tier allows for the account (an SMTP-only account lists `message_send`, not `envelope_list`); a dynamic-tier verb stays listed and answers `capability-missing` as a tool error; tests in tests/mcp.rs.
- [ ] Ungate the clap tree from cargo features, keep dispatch gated; confirm `json-schema` output is identical for `--no-default-features` and full builds.
- [ ] Generate the per-command backend support annotation into `--help` and the manual from the registry.
- [ ] Compatibility suite: every existing shared command's stdout (table + `--json`) unchanged against 2.x for each backend.
- [ ] `nix develop --command cargo build/test --bins`; reduced-feature builds; `cargo fmt`; clippy clean.
- [ ] MIGRATION.md note: new capability error replaces unknown-subcommand for unsupported verbs; nothing removed.
- [ ] Rewrite the `cairn/spec/commands.md` opening paragraph: the shared API is a capability union held coherent by one-meaning-per-verb, no longer the least-common-denominator intersection.
- [ ] Fold `delta.md` into `cairn/spec/commands.md`, `cairn/spec/backends.md` and `cairn/spec/mcp.md`; add `cairn/log` entry; mark change `landed`.

---
cairn: tasks
change: capability-shared-api
---

# Tasks

- [ ] Capability enum + per-backend static registry (one declaration site per backend, next to its adapter).
- [ ] Resolver: verb -> capability -> configured transport, with `--backend` as filter and explicit override; assert it reproduces today's `select_storage` order for every existing shared verb.
- [ ] Dynamic tier: per-client capability probe (IMAP `CAPABILITY`, JMAP session, ManageSieve `SIEVE`) consulted only after connection, only for verbs that need it.
- [ ] Capability error type: account, resolved backend, missing capability, protocol-specific alternative; non-zero exit, printed like every other error.
- [ ] Ungate the clap tree from cargo features, keep dispatch gated; confirm `json-schema` output is identical for `--no-default-features` and full builds.
- [ ] Generate the per-command backend support annotation into `--help` and the manual from the registry.
- [ ] Compatibility suite: every existing shared command's stdout (table + `--json`) unchanged against 2.x for each backend.
- [ ] `nix develop --command cargo build/test --bins`; reduced-feature builds; `cargo fmt`; clippy clean.
- [ ] MIGRATION.md note: new capability error replaces unknown-subcommand for unsupported verbs; nothing removed.
- [ ] Rewrite the `cairn/spec/commands.md` opening paragraph: the shared API is a capability union held coherent by one-meaning-per-verb, no longer the least-common-denominator intersection.
- [ ] Fold `delta.md` into `cairn/spec/commands.md` and `cairn/spec/backends.md`; add `cairn/log` entry; mark change `landed`.

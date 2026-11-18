---
cairn: tasks
change: one-trust-policy-per-account
---

- [x] src/config.rs: an account-level `tls`, merged into each backend's `TlsConfig` field by field (`inherit_tls`, `TlsConfig::inherit`).
- [x] ~~src/wizard/search.rs: discovery and probes take the given trust instead of `discovery_tls`'s fixed profile~~ Dropped: the wizard left (the-wizard-leaves, 2026-11-11), and with it the only discovery Himalaya ran.
- [x] `account check`: the effective trust per transport, a `tls` object per row (`AccountCheckReport`).
- [x] Tests: `backends_inherit_the_account_tls_field_by_field` (one `cert` at account level reaching IMAP, SMTP and JMAP, a transport override winning; failing on e440b4e4, the key refused), tests/account.rs `an_account_check_names_the_trust_of_each_transport` (failing on e440b4e4).
- [x] Spec: config.md and commands.md; config.sample.toml; fold, log, CHANGELOG.

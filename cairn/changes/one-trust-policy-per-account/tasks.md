---
cairn: tasks
change: one-trust-policy-per-account
---

- [ ] src/config.rs: an account-level `tls`, merged into each backend's `TlsConfig` field by field
- [ ] src/wizard/search.rs: discovery and probes take the given trust instead of `discovery_tls`'s fixed profile
- [ ] `account check`: the effective trust per transport
- [ ] Tests: one `cert` at account level reaching IMAP, SMTP and JMAP; a transport override winning; discovery against a host signed by a custom CA
- [ ] Spec: config.md and wizard.md; fold, log, CHANGELOG

---
cairn: tasks
change: one-trust-policy-per-account
---

- [ ] src/config.rs: an account-level `tls`, merged into each backend's `TlsConfig` field by field
- [x] ~~src/wizard/search.rs: discovery and probes take the given trust instead of `discovery_tls`'s fixed profile~~ Dropped: the wizard left (the-wizard-leaves, 2026-11-11), and with it the only discovery Himalaya ran.
- [ ] `account check`: the effective trust per transport
- [ ] Tests: one `cert` at account level reaching IMAP, SMTP and JMAP; a transport override winning
- [ ] Spec: config.md; fold, log, CHANGELOG

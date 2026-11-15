---
cairn: tasks
change: cli-alignment-leftovers
---

# Tasks

- [x] G2: thread one `SecretResolver` per account through `ImapClient::new` and `SmtpClient::new`: `EmailClient` holds it, so the lazy SMTP connect reuses what the IMAP session resolved ([#813](https://github.com/pimalaya/himalaya/issues/813))
- [x] ~~H1, H2: once pimalaya-cli ships them, drop src/wizard/configure.rs:103, :223, :278 and src/cli.rs:221~~ H1 dropped with the wizard (the-wizard-leaves, 2026-11-11); H2 done by the-first-run-welcome (2026-11-11), `resolve_account` (src/cli.rs:178) now being `Config::load` then pimalaya-config's shared `take_account`.
- [x] CHANGELOG under [Unreleased]
- [x] Fold into the spec; log; land

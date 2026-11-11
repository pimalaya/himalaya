---
cairn: tasks
change: cli-alignment-leftovers
---

# Tasks

- [ ] G2: thread one `SecretResolver` per account through `ImapClient::new` and `SmtpClient::new` (src/imap/client.rs:44, src/smtp/client.rs:45), or record why the lazy SMTP connect still rules it out
- [x] ~~H1, H2: once pimalaya-cli ships them, drop src/wizard/configure.rs:103, :223, :278 and src/cli.rs:221~~ H1 dropped with the wizard (the-wizard-leaves, 2026-11-11); H2 done by the-first-run-welcome (2026-11-11), `resolve_account` (src/cli.rs:178) now being `Config::load` then pimalaya-config's shared `take_account`.
- [ ] CHANGELOG under [Unreleased]
- [ ] Fold into the spec; log; land

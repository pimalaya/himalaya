---
cairn: tasks
change: cli-alignment-leftovers
---

# Tasks

- [ ] G2: thread one `SecretResolver` per account through `ImapClient::new` and `SmtpClient::new` (src/imap/client.rs:44, src/smtp/client.rs:45), or record why the lazy SMTP connect still rules it out
- [ ] H1, H2: once pimalaya-cli ships them, drop src/wizard/configure.rs:103, :223, :278 and src/cli.rs:221
- [ ] CHANGELOG under [Unreleased]
- [ ] Fold into the spec; log; land

---
cairn: tasks
change: smtp-send-dsn
---

- [x] io-smtp: MAIL and RCPT parameters on `SmtpMessageSendOptions`
- [x] Keep the EHLO capabilities on `SmtpClient`
- [x] `--notify`, `--ret`, `--envid` on `smtp send`, refused without `DSN`
- [x] Verified against a scripted SMTP server, with and without `DSN`
- [x] Release io-smtp 0.6.0, drop the `[patch.crates-io]` entry

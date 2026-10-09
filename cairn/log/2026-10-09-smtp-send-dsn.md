---
cairn: log
change: smtp-send-dsn
landed: 2026-10-09
---

# smtp send requests delivery status notifications

Requested in [#775] by @snrmwg. `himalaya smtp send` takes `--notify` (comma-separated `never`, `success`, `failure`, `delay`, sent as `NOTIFY` on every `RCPT TO`), `--ret full|hdrs` and `--envid <ID>` (sent as `RET` and `ENVID` on `MAIL FROM`). `SmtpClient` now keeps the capability lines of the last EHLO, and the command fails before `MAIL FROM` when `DSN` is not among them. `never` combined with another condition is refused, as is an `ENVID` outside printable ASCII, holding `+` or `=`, or longer than 100 characters, so it never needs xtext encoding. The shared `message send` is unchanged.

Built on io-smtp 0.6, whose `SmtpMessageSendOptions` gained `mail_parameters` and `rcpt_parameters`.

Verified against a scripted SMTP server: with `DSN` announced the server read `MAIL FROM:<me@example.org> RET=HDRS ENVID=abc123` and `NOTIFY=SUCCESS,FAILURE` on both recipients; without it, `--notify` and `--ret` each failed before `MAIL FROM`; with no flag the transaction is unchanged.

Capabilities moved: **backends** (Sending transport: `smtp send` requests DSN).

[#775]: https://github.com/pimalaya/himalaya/issues/775

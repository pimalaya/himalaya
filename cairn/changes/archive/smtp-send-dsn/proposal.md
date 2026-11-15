---
cairn: change
id: smtp-send-dsn
status: landed
created: 2026-10-09
---

# smtp send requests delivery status notifications

## Why

[#775] asks for delivery status notifications (RFC 3461) at send time, for a Neovim plugin on the CLI. The only route was `smtp raw` with a hand-written transaction.

## What

- `smtp send` gains `--notify <WHEN>` (comma-separated `never`, `success`, `failure`, `delay`, on every `RCPT TO`), `--ret <full|hdrs>` and `--envid <ID>` (on `MAIL FROM`).
- The command fails before `MAIL FROM` when the server does not announce `DSN`: dropping a request the user asked for would be worse.
- `SmtpClient` keeps the EHLO capabilities instead of discarding them.
- The shared `message send` stays out: Gmail and Graph have no DSN.
- Needs io-smtp 0.6 (`SmtpMessageSendOptions::{mail_parameters, rcpt_parameters}`).

[#775]: https://github.com/pimalaya/himalaya/issues/775

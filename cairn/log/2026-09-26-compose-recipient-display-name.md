---
cairn: log
change: compose-recipient-display-name
landed: 2026-09-26
---

# Recipients keep their display name apart

Reported for the sender in #727 and fixed there for `--from` only. `message compose`, `reply` and `forward` still handed each `--to`, `--cc` and `--bcc` value to the MIME builder as a bare address, so `--to 'Alice <alice@example.org>'` wrote `To: <Alice <alice@example.org>>`, which no SMTP server accepts.

Each recipient now goes through the same mailbox parsing as the sender, so the display name is encoded apart from the address. A value with no email address is rejected with a clear error instead of producing an invalid header.

Spec unchanged.

---
cairn: log
change: compose-recipient-address-list
landed: 2026-09-26
---

# Recipient flags take address lists

Follow-up to compose-recipient-display-name. `--to`, `--cc` and `--bcc` of `message compose`, `reply` and `forward` were split on commas by the CLI before parsing, so `"Doe, Alice" <alice@example.org>` reached the parser in two broken halves. The flags no longer split values themselves: each value is parsed as an RFC 5322 address list and every mailbox it carries is kept, so both repeating the flag and `a@x, b@y` still work.

Spec unchanged.

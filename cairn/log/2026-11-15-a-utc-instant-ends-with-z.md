---
cairn: log
change: a-utc-instant-ends-with-z
landed: 2026-11-15
---

# A UTC instant ends with `Z`

Task H10 of the family's cli-alignment-2026-11-15 plan (finding F14). `Z`, the spelling of pimalaya-cli's cairn/spec/output.md and of most of Himalaya (the envelope dates through chrono's serde, the `vacation` dates), is now the only one.

## What landed

- src/shared/message/read.rs `rfc3339`: the read view's `headers.date` writes a zero offset `Z`, any other as found.
- src/imap/fetch.rs: `internalDate` written as chrono's serde writes it (`SecondsFormat::AutoSi`, `Z`), where `to_rfc3339` wrote `+00:00`.
- Tests: the read view's date test (it read `+00:00`), and tests/local_stores.rs through the binary.
- CHANGELOG (**BREAKING**) and MIGRATION.md: `imap fetch --internal-date` printed `+00:00` since v2.

## Capabilities moved

- [commands](../spec/commands.md): *An instant has one spelling*, and `headers.date` in *A read message is a designed view*.

---
cairn: log
change: imap-raw-text-base64
landed: 2026-11-19
---

# imap raw prints an 8-bit response in base64 in text

Owner ruling 2026-11-19: the text form of `imap raw` no longer decodes a response that is not UTF-8 lossily.

## What landed

- src/imap/raw.rs: `split_reply` pushes the line `undecoded (base64): <base64>` for a response that is not UTF-8, the base64 the one the `undecoded` event carries (its octets, literals and final CRLF included); `ImapRawEvent::Undecoded` displays the same line. No existing text convention for binary output was found (byte writers print raw bytes, `--json` base64), so the prefix is new; no well-formed IMAP response opens with it.
- Test `the_text_form_prints_an_8bit_response_in_base64` failed against the lossy line (U+FFFD found) before the change.
- CHANGELOG: the unreleased breaking `imap raw` line amended. MIGRATION: the v2 to v3 `imap raw` line amended.

## Capabilities moved

- [commands](../spec/commands.md): *Raw passthrough is byte-verbatim*, its text form, and the scenario *An 8-bit literal in the text form*.

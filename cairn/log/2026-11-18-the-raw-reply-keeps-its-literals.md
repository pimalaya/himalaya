---
cairn: log
change: the-raw-reply-keeps-its-literals
landed: 2026-11-18
---

# The raw reply keeps its literals

Finding HIM-18 (local part) of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/imap/raw.rs: `split_responses` stops at a literal it cannot measure, the rest one `unparsed` event, `ImapRawOutput::unframed` exiting 3 with a warning.
- Tests `a_literal_that_is_no_utf8_does_not_panic` and `a_body_line_is_never_a_completion`, both failing before the fix.
- CHANGELOG (Fixed).
- Still open upstream: io-imap 0.11.2 returns the reply through `from_utf8_lossy`; the byte framing and the base64 schema change wait on it.

## Capabilities moved

- [commands](../spec/commands.md): *Raw passthrough is byte-verbatim*.

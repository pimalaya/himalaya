---
cairn: change
id: the-colon-path-is-no-message
status: landed
created: 2026-11-18
---

# The colon path is no message

## Why

Finding HIM-5 of the family's cli-family-analysis-2026-11-15 plan, rechecked on the live head 2026-11-18. `himalaya message add -m Sent '/nonexistent/Mail/cur/1700000000.host:2,S'` exited 0 with a receipt, the stored message being the path: `opens_on_a_field` accepted any printable run before the first colon, and every Maildir file name holds `:2,`, as a Windows path holds `C:`. The escapes of an inline message were also decoded before the file test, so an existing file under a path holding a backslash then `n` or `r` (`C:\Users\nick\...`) was not found. cairn/spec/commands.md says a mistyped path is refused.

## What

- `opens_on_a_field` (src/shared/message/arg.rs) holds the field name to letters, digits, `-`, `_` and `.`, and reads a one-letter name followed by `/` or `\` as a Windows drive.
- `resolve` looks the file up under the positional as typed, then decodes the escapes of an inline message.

---
cairn: delta
change: the-colon-path-is-no-message
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Raw message input is shared
A positional naming no file SHALL be the message only when it opens on a header field, a name of letters, digits, `-`, `_` and `.` then a colon, a one-letter name followed by `/` or `\` being a Windows drive, so a mistyped path, a Maildir file name holding `:2,` included, is refused rather than stored. The file SHALL be looked up under the positional as typed, before the `\n` and `\r` escapes of an inline message are decoded.

## REMOVED Requirements

None.

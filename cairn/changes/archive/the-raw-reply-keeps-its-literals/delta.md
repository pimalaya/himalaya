---
cairn: delta
change: the-raw-reply-keeps-its-literals
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Raw passthrough is byte-verbatim
A literal holding a U+FFFD has no knowable octet count, so `imap raw` SHALL frame the reply up to the response announcing it and print that response and everything after it as one `unparsed` event (one block in text), never splitting a line of the literal into a response, then the completions read before it, and exit 3.

## REMOVED Requirements

None.

---
cairn: delta
change: reply-flags-answered
---

## ADDED Requirements

### Requirement: A sent reply or forward flags its source
`message reply --send` SHALL add the `answered` flag (RFC 3501 §2.3.2) to the source message once the send has succeeded. A flag failing after a successful send SHALL fail the command with an error stating that the message was sent. `message forward --send` SHALL add the `forwarded` flag (RFC 5788) to the source the same way, best-effort: a backend refusing the keyword SHALL be logged as a warning and SHALL NOT fail the command. A reply or forward only printed or saved SHALL leave the source untouched.

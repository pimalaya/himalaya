---
cairn: change
change: imap-fetch-body
---

# Delta

## ADDED Requirements

### Requirement: IMAP fetch can return whole messages
`imap fetch --body` SHALL add `BODY.PEEK[]` (RFC 3501 §6.4.5) to the requested data items, so a caller downloads every message of a sequence set over one session instead of one `message read` per message. The fetch SHALL NOT set `\Seen`. The JSON output SHALL carry the message octets byte-exact as the standard Base64 of the `body` field; the plain rendering SHALL print the size only. `--body` SHALL NOT imply `--envelope`.

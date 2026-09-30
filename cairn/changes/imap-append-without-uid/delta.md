---
cairn: change
change: imap-append-without-uid
---

# Delta

## MODIFIED Requirements

### Requirement: Append and search gaps
Gmail and Graph SHALL NOT implement `add_message` (neither API has an append) and SHALL NOT implement shared `search_envelopes`. IMAP, JMAP, Maildir and m2dir implement search (see the search capability).

The shared `add_message` result SHALL carry an optional id, absent only when an IMAP or JMAP server acknowledges the write without reporting one. An IMAP `APPEND` acknowledged by the server SHALL stay a success when neither `APPENDUID` (RFC 4315) nor the fallback `Message-ID` search yields a UID, and a requested send SHALL go on. An `APPEND` rejected by the server SHALL stay an error.

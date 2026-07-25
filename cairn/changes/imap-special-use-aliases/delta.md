---
cairn: delta
change: imap-special-use-aliases
---

## MODIFIED Requirements

### Requirement: IMAP special-use is inbox-only for now
IMAP special-use alias discovery SHALL cover the reserved `INBOX` plus the Sent/Drafts/Trash/Junk/Archive roles, read from LIST `RETURN (SPECIAL-USE)` (RFC 6154) over the reused test connection. When the listing is empty or fails, only `INBOX` is pinned.

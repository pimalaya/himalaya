---
cairn: delta
change: imap-special-use-aliases
---

## MODIFIED Requirements

### Requirement: IMAP special-use is inbox-only for now
IMAP SHALL read the mailbox roles from LIST `RETURN (SPECIAL-USE)` (RFC 6154): `INBOX` plus the Sent/Drafts/Trash/Junk/Archive attributes.

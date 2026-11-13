---
cairn: delta
change: imap-special-use-aliases
---

## MODIFIED Requirements

### Requirement: IMAP special-use roles come from a plain LIST
IMAP SHALL read the mailbox roles from LIST `RETURN (SPECIAL-USE)` (RFC 6154), on top of the attributes a plain LIST carries: `INBOX` plus the Sent/Drafts/Trash/Junk/Archive attributes, so a server marking its roles only when asked shows them too.

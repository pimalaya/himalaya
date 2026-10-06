---
cairn: delta
change: message-read-invalid-recipients
---

# Delta

## ADDED Requirements

### Requirement: The read view lists unreadable address entries
`headers.invalid` SHALL be an object `{to, cc, bcc, replyTo, from, sender}`, each a list of the entries of that header that are not readable as an address and so are absent from its list, as decoded text (encoded words decoded, trimmed), in header order, empty when none. A group name SHALL NOT be an entry, nor an empty group. `message read --json` and `message parse --json` SHALL print the same.

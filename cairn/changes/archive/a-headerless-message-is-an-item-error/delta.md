---
cairn: delta
change: a-headerless-message-is-an-item-error
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Each backend names the items it could not serve
In backends.md, the Maildir item reads "a file that cannot be read, or holding no header field (no line opening on an RFC 5322 field name and a colon), SHALL be `message-unreadable` with its id, never a row", and an item follows it:

- mbox: a message holding no header field SHALL be `message-unreadable` with its id, never a row.

### Requirement: mbox reads go through a cache
In backends.md, the cache keeps the envelope of every message already parsed, "or that it holds no header field".

### Requirement: A command acting on several items lists what failed
In commands.md, mbox joins the listings printing their errors on their first page only, and the scenario *A file holding no header field* is added. In the `message-unreadable` row of the code table, a message holding no header field (Maildir, mbox) joins the causes.

## REMOVED Requirements

None.

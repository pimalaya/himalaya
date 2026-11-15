---
cairn: change
id: a-gmail-or-graph-read-is-scoped-to-its-mailbox
status: landed
created: 2026-11-15
---

# A Gmail or Graph read is scoped to its mailbox

## Why

Task H33 of the family's cli-alignment-2026-11-15 plan, left open by the-smaller-live-findings (task H32): a Gmail or Microsoft Graph `message read -m <mailbox> <id>` read the message whatever mailbox `-m` named, Gmail and Graph ids naming a message account-wide, where IMAP, and JMAP since that change, fail `message-not-found` naming the mailbox. `message reply|forward` and `attachment list|download` read their source the same way. The writes (`flag`, `message copy|move|delete`) were scoped already.

## What

- Gmail: the `messages.get` (format RAW) bringing the bytes brings the message's `labelIds` too; a message not carrying the mailbox's label fails `message-not-found`, naming the mailbox, before `--seen` writes anything. The mailbox is resolved through the label listing only when it is not one of the message's label ids: a label id (`INBOX`, `Label_12`) costs nothing more, a name or a role (`inbox`, the default of `-m`) one `labels.list`, as a JMAP read of a role costs one `Mailbox/get`.
- Graph: `GET /messages/{id}/$value` carries no property, so the read first asks `GET /messages/{id}?$select=id,parentFolderId`, one request more, and fetches the bytes only when that folder is the one the mailbox resolves to, a well-known name included; another folder fails `message-not-found`, naming the mailbox. The mailbox is resolved through the folder listing only when it is not the message's folder id already: a name or a role (`inbox`, the default of `-m`) costs the listing every other unified verb pays (the well-known lookups and the folder pages), a folder id nothing more. An id Graph does not know fails `message-not-found` as before, from either request.
- Same code and sentence as JMAP: ``Message `<id>` not found in `<mailbox>` ``.

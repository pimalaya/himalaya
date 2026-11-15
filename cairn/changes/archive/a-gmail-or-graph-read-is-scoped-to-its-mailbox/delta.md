---
cairn: delta
change: a-gmail-or-graph-read-is-scoped-to-its-mailbox
---

# Delta

## ADDED Requirements

### Requirement: A Gmail read is scoped to the mailbox given
The Gmail backend SHALL read one message (`message read`, and the source of `message reply|forward` and `attachment list|download`) only when it carries the label of the mailbox given, reading its `labelIds` in the one `messages.get` (format RAW) bringing its bytes: a message not carrying the label SHALL fail `message-not-found`, naming the mailbox, as on IMAP and JMAP, though a Gmail id names a message account-wide, and `--seen` SHALL change nothing. The mailbox SHALL be resolved through the label listing only when it is not among the message's label ids.

#### Scenario: An inbox message read under the sent mailbox
- GIVEN a Gmail account whose message `m1` carries the `INBOX` label alone
- WHEN `himalaya message read -m sent m1` runs
- THEN the command fails with the code `message-not-found`, naming `sent`, where it printed the message

### Requirement: A Graph read is scoped to the mailbox given
The Graph backend SHALL read one message (`message read`, and the source of `message reply|forward` and `attachment list|download`) only when its `parentFolderId` is the folder the mailbox given resolves to, a well-known name included, read through `GET /messages/{id}?$select=id,parentFolderId` before `GET /messages/{id}/$value`, which carries no property: a message of another folder SHALL fail `message-not-found`, naming the mailbox, as on IMAP and JMAP, though a Graph id names a message account-wide, its bytes not fetched and `--seen` changing nothing. The mailbox SHALL be resolved through the folder listing only when it is not the message's folder id. An id Graph answers 404 or 400 `ErrorInvalidIdMalformed` for, at either request, SHALL fail `message-not-found`.

#### Scenario: An inbox message read under the sent mailbox
- GIVEN a Graph account whose message `m1` the Inbox holds
- WHEN `himalaya message read -m sentitems m1` runs
- THEN the command fails with the code `message-not-found`, naming `sentitems`, where it printed the message

## MODIFIED Requirements

None.

## REMOVED Requirements

None.

---
cairn: log
change: a-gmail-or-graph-read-is-scoped-to-its-mailbox
landed: 2026-11-15
---

# A Gmail or Graph read is scoped to its mailbox

Task H33 of the family's cli-alignment-2026-11-15 plan, left open by the-smaller-live-findings: a Gmail or Graph `message read -m <mailbox>`, and the source read of `message reply|forward` and `attachment list|download`, read a message the mailbox does not hold, where IMAP and JMAP fail `message-not-found`.

## What landed

- src/gmail/backend.rs `get_message`: the `labelIds` of the RAW `messages.get` must hold the mailbox's label, resolved through the label listing only when the mailbox is not one of them; else `message-not-found`, ``Message `<id>` not found in `<mailbox>` ``; test `a_read_of_a_message_another_label_holds_is_not_found`.
- src/msgraph/backend.rs `get_message`: `GET /messages/{id}?$select=id,parentFolderId` first, one request more since `$value` carries no property, the bytes fetched only when the folder is the mailbox's, resolved through the folder listing only when the mailbox is not that folder id; `read_error` codes an unknown id at either request; tests `a_read_of_a_message_another_folder_holds_is_not_found` and `a_read_of_an_unknown_id_is_not_found`.
- src/error.rs: `message_not_found` built for Gmail and Graph alone too.

## Capabilities moved

- [backends](../spec/backends.md): *A Gmail read is scoped to the mailbox given* and *A Graph read is scoped to the mailbox given* added, each with a scenario.

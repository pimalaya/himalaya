---
cairn: tasks
change: a-gmail-or-graph-read-is-scoped-to-its-mailbox
---

# Tasks

- [x] src/gmail/backend.rs `get_message`: the label check over the `labelIds` of the RAW `messages.get`, the mailbox resolved only when it is not one of them; test `a_read_of_a_message_another_label_holds_is_not_found` over the scripted Gmail server.
- [x] src/msgraph/backend.rs `get_message`: `GET /messages/{id}?$select=id,parentFolderId` before `$value`, the folder compared with the mailbox, resolved only when it is not the message's folder id; `read_error` codes an unknown id at either request; tests `a_read_of_a_message_another_folder_holds_is_not_found` and `a_read_of_an_unknown_id_is_not_found` over the scripted Graph server.
- [x] src/error.rs: `message_not_found` built for Gmail and Graph alone too.
- [x] Fold into cairn/spec/backends.md; CHANGELOG; MIGRATION.md; the log entry.

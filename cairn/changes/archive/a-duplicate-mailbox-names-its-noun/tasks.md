---
cairn: tasks
change: a-duplicate-mailbox-names-its-noun
---

# Tasks

- [x] src/error.rs: `ErrorCode::MailboxDuplicate`, `mailbox_duplicate` (a local refusal) and `existing_mailbox` (a server's answer kept in `sources`); `ItemDuplicate` codes a message alone.
- [x] src/pimdir/backend.rs `create_mailbox`: a listed or queued mailbox fails `mailbox-duplicate`; test `a_mailbox_the_store_lists_is_not_created_again`.
- [x] src/maildir/client.rs `refuse_existing`, used by src/maildir/backend.rs `create_mailbox` and src/maildir/create.rs: a folder there fails `mailbox-duplicate`, a file uncoded; test `an_existing_folder_is_not_created_again`.
- [x] src/mbox/backend.rs `create_mailbox`: io-mbox's `MboxCreateError::AlreadyExists` on a file fails `mailbox-duplicate`; test `an_existing_mbox_is_not_created_again`.
- [x] src/imap/backend.rs `create_refusal`: `ALREADYEXISTS`, or no code and the mailbox listed; test `a_create_of_an_existing_mailbox_is_a_duplicate` over the scripted IMAP server.
- [x] src/jmap/backend.rs `create_mailbox` and `holds_sibling`; test `a_create_of_an_existing_mailbox_is_a_duplicate` over the scripted JMAP server.
- [x] src/gmail/backend.rs `create_mailbox` and `names_existing_label`; test `a_create_of_an_existing_label_is_a_duplicate`.
- [x] src/msgraph/backend.rs `create_mailbox`, `holds_child` and `names_existing_folder`; test `a_create_of_an_existing_folder_is_a_duplicate`.
- [x] tests/local_stores.rs `a_mailbox_created_twice_is_a_duplicate`: the binary, unified and raw, on Maildir and mbox, exit 1.
- [x] Fold into cairn/spec/commands.md and cairn/spec/backends.md; CHANGELOG; MIGRATION.md; the log entry.

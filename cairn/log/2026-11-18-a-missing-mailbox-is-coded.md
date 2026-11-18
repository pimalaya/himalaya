---
cairn: log
change: a-missing-mailbox-is-coded
landed: 2026-11-18
---

# A missing mailbox is coded

Findings F4, J5, G6 and M4 of the live provider run of 2026-11-18 (cairn/spec/testing/).

## What landed

- src/imap/backend.rs: every command addressing a mailbox (`SELECT`, `APPEND`, `COPY`, `MOVE`, `RENAME`, `DELETE`) explains a `NO`: `NONEXISTENT` or `TRYCREATE` is `mailbox-not-found` at once, a plain `NO` costs one `LIST` after it, anything else keeps the server's refusal and sends none; a `STATUS` refused `NONEXISTENT` is an `errors` entry so coded.
- src/jmap/client.rs, src/gmail/client.rs, src/msgraph/client.rs: an unknown name, id or role is `mailbox-not-found`, Graph's 404 kept in `sources`.
- src/error.rs: `unknown_mailbox`.
- Tests, each failing on 3b9f9127: tests/missing_mailbox.rs (`a_missing_mailbox_is_coded`, `a_copy_to_a_missing_mailbox_is_coded`, `a_listed_mailbox_keeps_the_refusal`), `jmap::backend::tests::send::a_missing_mailbox_is_coded`, `gmail::client::tests::a_missing_label_is_coded`, `msgraph::client::tests::a_missing_folder_is_coded`.
- CHANGELOG (Changed, breaking), MIGRATION (v2 to v3).

## Capabilities moved

- [commands](../spec/commands.md): *A server's refusal codes the whole command*, *The JSON contract is described and versioned* (the code table).
- [backends](../spec/backends.md): *Graph folders are named by path*.

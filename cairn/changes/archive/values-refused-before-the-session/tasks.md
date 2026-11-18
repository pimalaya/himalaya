---
cairn: tasks
change: values-refused-before-the-session
---

# Tasks

- [x] src/imap/utils.rs `sequence_set_parser` on src/imap/fetch.rs, flag/store.rs, message/copy.rs, message/move.rs; test `a_bad_sequence_set_is_refused_by_the_grammar`.
- [x] src/jmap/submission/create.rs `requires`; src/msgraph/messages/update.rs required `change` group.
- [x] src/gmail/request.rs, src/msgraph/request.rs: method possible values, `path_parser`.
- [x] src/smtp/send.rs `check`, src/smtp/cli.rs, src/cli.rs before `account.run`.
- [x] Test tests/early_refusal.rs `a_value_the_command_refuses_is_refused_before_the_session`.
- [x] src/imap/mailbox/status.rs: the three keys removed; test `the_status_prints_the_items_it_asks_for`.
- [x] Fold into cairn/spec/commands.md, CHANGELOG, MIGRATION.

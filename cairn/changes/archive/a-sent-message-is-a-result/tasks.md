---
cairn: tasks
change: a-sent-message-is-a-result
---

# Tasks

- [x] src/shared/message/handler.rs: `Outcome::Unsaved`, `route_unprinted`, `MessageRouteOutput.errors` and `print`.
- [x] src/shared/client.rs: `SentCopy::Unfiled` for Gmail's filing after the send.
- [x] src/shared/message/add.rs: `MessageAddOutput.errors`.
- [x] src/shared/message/reply.rs: the answered flag's failure as an item error.
- [x] Tests tests/send_outcome.rs `a_copy_refused_after_the_send_is_an_item_error`, `a_reply_whose_source_is_not_flagged_prints_one_receipt`.
- [x] Fold into cairn/spec/commands.md, CHANGELOG, MIGRATION.

---
cairn: log
change: a-sent-message-is-a-result
landed: 2026-11-18
---

# A sent message is a result

Finding HIM-11 of the family's cli-family-analysis-2026-11-15 plan, ruled by the owner on 2026-11-18 (D26).

## What landed

- src/shared/message/handler.rs, src/shared/client.rs, src/shared/message/add.rs, src/shared/message/reply.rs: a copy refused after the send, and a reply's answered flag failing after it, are item errors of the one receipt, `sent: true`, exit 3; `MessageRouteOutput` and `MessageAddOutput` print `errors`.
- Tests tests/send_outcome.rs `a_copy_refused_after_the_send_is_an_item_error` and `a_reply_whose_source_is_not_flagged_prints_one_receipt`, failing on 72c1c0ca.
- CHANGELOG (Changed, breaking), MIGRATION (v2 to v3).

## Capabilities moved

- [commands](../spec/commands.md): *Saving a sent message follows the send*, *A sent reply or forward flags its source*, *A command acting on several items lists what failed*.

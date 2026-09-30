---
cairn: log
change: pimdir-queued-send
landed: 2026-09-30
---

# A pimdir account sends through the queue

`message send`, and every composer with `--send`, now queues a `submit` intent on a pimdir account instead of sending over the account's SMTP transport, which such an account no longer uses. Whatever owns the store performs it through its own channel and credentials; Neverest is the owner implementing it today.

The row carries the `v: 1` `submit` payload (`object`, `from`, `rcpts`, `subject`) and pins the body, stored as given with its `Bcc:` field. It is filed under the `--save` mailbox, else `mailbox.alias.sent`, and refused with neither rather than create a collection. The envelope derivation moved into `email::submission::SubmissionEnvelope`, which the SMTP adapter now calls too.

The send confirmation became `MessageRouteOutput`, the `{message}` object it was plus `queueId` when a send was queued, and `message add --send` gained the same field. `pimdir queue list` shows queued sends beside queued creates, with an ACTION column, and the envelope listing counts both.

Capability moved: **backends** (pimdir sends by queueing a submit intent, the queue view shows queued sends, sending transport).

The payload shape is pinned in `a_sent_message_is_one_submit_row_with_its_envelope`; a test decoding it with an owner's own type (Neverest's `SubmitMeta`) would need the two crates side by side and was not written.

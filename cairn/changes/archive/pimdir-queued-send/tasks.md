---
cairn: tasks
change: pimdir-queued-send
---

- [x] Review the three decisions in the proposal
- [x] Move the envelope derivation (`From:` to reverse path, `To:`/`Cc:`/`Bcc:` to recipients) out of `src/smtp/backend.rs` into one function the SMTP adapter and the pimdir backend both call
- [x] `PimdirBackend::send_message`: stage the blob, enqueue `submit` with the `v: 1` payload, return the row id
- [x] Anchor on the `--save` mailbox, else `mailbox.alias.sent`, else refuse naming the alias
- [x] `EmailClient::send_message` routes a pimdir storage to its queue, ahead of SMTP
- [x] The send outcome carries the row id; text "queued for sending", JSON field for the id, `json_schema.rs` entry updated
- [x] `pimdir queue list` renders a queued `submit` from the body it pins, marked as a send
- [x] Test: a sent message is one `submit` row whose payload decodes to the derived envelope, Bcc in `rcpts`, body stored verbatim
- [x] Test: no `From:` or no recipient stages nothing
- [x] Test: no anchor stages nothing and names `mailbox.alias.sent`
- [ ] Test: Neverest's `SubmitIntent::envelope` decodes the payload Himalaya writes; not written, the payload shape is pinned on this side instead
- [x] CHANGELOG: pimdir sends through the queue; an `smtp` section on a pimdir account is no longer used
- [x] Fold the delta into `cairn/spec/backends.md`, log, `status: landed`

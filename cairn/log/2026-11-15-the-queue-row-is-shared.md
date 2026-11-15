---
cairn: log
change: the-queue-row-is-shared
landed: 2026-11-15
---

# The queue row is shared

Rulings D19 and D21 of the family's cli-alignment-2026-11-15 plan: `pimdir queue list` prints every row, pending or parked, in the family's one queue row plus `subject`, every key printed, where Himalaya printed a pending row as `{row, queuedAt, producer, send, envelope}`.

## What landed

- src/pimdir/queue/list.rs: `PimdirQueueRow` `{row, kind, collection, linkId, itemId, producer, queuedAt, attempts, unconfirmed, error, subject}` for `messages` and `parked`; the text prints the row, its state, when it was queued, its kind, then its subject or why it was parked.
- src/pimdir/backend.rs: `queued_rows`, `parked_rows` and `addressed` (the link id an add creates, the public id of the message a row acts on, the subject of the body pinned, else of that message, else a parked submit's payload subject), parked payloads decoded through io-pimdir's `action_from_payload`; `PimdirQueued`, `PimdirQueuedEnvelope` and `queued_from_action` removed, and with them `send` and `envelope`.
- Tests: `a_pending_and_a_parked_row_keep_one_shape`, `a_queued_creation_names_its_link_id_and_its_subject`, `a_row_acting_on_a_message_names_its_public_id`, and the rows asserted in three existing pimdir tests.

## Capabilities moved

- [backends](../spec/backends.md): *The pimdir subcommand reads and retracts the queue* states the row, with the scenarios *A queued creation is listed from its body* and *A parked move names its message*; *The pimdir queue view shows parked rows* and *The queue view shows queued sends* follow it.
- [commands](../spec/commands.md): *Text output is plain text* gives the new `pimdir queue list` line.

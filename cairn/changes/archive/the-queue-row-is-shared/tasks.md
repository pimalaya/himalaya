---
cairn: tasks
change: the-queue-row-is-shared
---

# Tasks

- [x] src/pimdir/queue/list.rs: `PimdirQueueRow` for both lists, the text layout; test `a_pending_and_a_parked_row_keep_one_shape`.
- [x] src/pimdir/backend.rs: `queued_rows`, `parked_rows` and `addressed` replace `queued_envelopes`, `PimdirQueued`, `PimdirQueuedEnvelope` and `queued_from_action`; tests `a_queued_creation_names_its_link_id_and_its_subject`, `a_row_acting_on_a_message_names_its_public_id`, and the queue rows asserted in `a_sent_message_is_one_submit_row_with_its_envelope`, `a_parked_row_is_listed_and_cancelled` and `a_move_over_a_conflicted_message_stays_listed_and_parks`.
- [x] Fold into cairn/spec/backends.md and cairn/spec/commands.md; CHANGELOG; MIGRATION.md; the log entry.

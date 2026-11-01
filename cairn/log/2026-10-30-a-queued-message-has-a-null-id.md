---
cairn: log
change: a-queued-message-has-a-null-id
landed: 2026-10-30
---

# A queued message in the queue view has a `null` id

From the pimdir stable-release check of 2026-10-30 against the rule of pimdir-queue-visibility (cairn/log/2026-08-27-pimdir-queue-visibility.md), confirmed that day for every pimdir frontend: no placeholder in an id field. `pimdir queue list --json` printed `"id": ""` in each queued message's `envelope`. The queue view now carries its own envelope type, `PimdirQueuedEnvelope` (src/pimdir/backend.rs), with the shared envelope's keys and an `id: Option<String>`, `None` for every queued creation or send; the shared `Envelope` keeps its `String` id. The row id stays the row's own `id` and the ROW column, and the plain table, which never showed an envelope id, is unchanged.

Test: `a_queued_creation_renders_as_mail_with_a_null_id` serialises the envelope `queued_from_action` builds and checks `id` is `null`; on the old code it failed with `left: String("")`, `right: Null`.

Capabilities moved: **backends** (The pimdir subcommand reads and retracts the queue).

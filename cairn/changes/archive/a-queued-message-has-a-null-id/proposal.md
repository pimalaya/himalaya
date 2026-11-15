---
cairn: change
id: a-queued-message-has-a-null-id
status: landed
created: 2026-10-30
---

# A queued message in the queue view has a `null` id

## Why

The rule for queued creations, settled in pimdir-queue-visibility (cairn/log/2026-08-27-pimdir-queue-visibility.md) and confirmed for every pimdir frontend on 2026-10-30, is that no placeholder goes in an id field: no `q<id>`, no `0`, no `""`, and `null` when a listing shows a queued creation at all. Checked on 2026-10-30 for the pimdir stable release: a create returns the link id and the queue view puts the row id under ROW, but `pimdir queue list --json` renders each message's `envelope` with `"id": ""` (src/pimdir/backend.rs `queued_from_action`, asserted by `a_queued_creation_renders_as_mail_with_no_id`). That is the empty placeholder the rule excludes.

## What

- `pimdir queue list --json` prints `"id": null` in a queued message's `envelope`.
- The queue view lives in the `pimdir` namespace, so it gets its own envelope type, `PimdirQueuedEnvelope`, with the shared envelope's keys and an `id: Option<String>`, `None` for a queued creation or send. The shared `Envelope` is not touched: its `id` stays a `String`, every backend having one for every message it lists (shared-api-lcd).
- The row id stays where it is, the row's own `id` and the ROW column. The plain table is unchanged: it never showed an envelope id, so there is no cell to empty.

## Decision

The open question of the draft (an envelope with an optional id, or only the few fields the table renders) was settled by the orchestrator of the pimdir stable release on 2026-10-30: keep an `envelope` object with every key the shared envelope has, so a caller reading `envelope list` reads a queued message the same way, and only `id` differs.

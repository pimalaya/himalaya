---
cairn: tasks
change: pimdir-queue-visibility
---

- [x] io-pimdir `reader-role` and `overlay-page-is-total`, patched to git
- [x] `PimdirClient` reads through an overlaying `PimdirReader`
- [x] A staged flag, remove, move, copy and update show on the next read
- [x] A queued create is not listed as a message; `Envelope.id` stays a `String`
- [x] The envelope listing output reports the mailbox's queued creates and names `himalaya pimdir queue list`
- [x] `envelope search` reports none: a queued create is never matched against the query
- [x] `himalaya pimdir queue list`, rendering a create as mail: flags, subject, recipient, age from the row's `created_at`
- [x] `himalaya pimdir queue cancel <row>`, through the scoped owner operation, confirming unless `--yes`
- [x] A cancel refused as owned says a sync is running and the action may already have applied
- [x] `PimdirQueuedMessages` and `PimdirQueueCancelled` with `json_schema.rs` entries
- [x] `message save` confirmation unchanged
- [x] Test: a queued creation renders as mail with an empty id
- [x] Test: only a creation renders in the queue view

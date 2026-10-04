---
cairn: tasks
change: shared-output-free-of-pimdir
---

# Tasks

- [x] Remove `Noted`, `take_notes` and the `notes` field from the shared write outputs and their JSON schemas; log partial support as a warning in the pimdir client.
- [x] Remove `queueId` from `MessageRouteOutput` and `MessageAddOutput`, and the queue id from `Outcome`; log the row id in the pimdir backend.
- [x] Remove `queued` from `Envelopes` and `EmailClient::queued_messages`.
- [x] Fold the delta into [cairn/spec/backends.md](../../spec/backends.md); write the log entry.

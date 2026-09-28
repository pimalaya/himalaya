---
cairn: tasks
change: imap-move-without-move
---

# Tasks

- [x] src/imap/client.rs: `supports_move()`, alongside `supports_uidplus()`.
- [x] src/imap/backend.rs: `move_messages` matches on `supports_move()` and `supports_uidplus()`; `UID MOVE`, or `UID COPY` + `\Deleted` + `UID EXPUNGE` of the `COPYUID` source UIDs, or the same copy and flag with the expunge skipped when UIDPLUS is missing.
- [x] Parse the requested UIDs once and clone the set where needed.
- [x] Manual check against a server without MOVE (OVH Dovecot): `message move` and `message delete` succeed, and no other `\Deleted` message in the source is expunged.
- [x] CHANGELOG entry.
- [x] Fold the delta into [cairn/spec/backends.md](../../spec/backends.md); write the log entry.

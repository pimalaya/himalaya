---
cairn: tasks
change: imap-move-without-move
---

# Tasks

- [x] src/imap/client.rs: `supports_move()`, alongside `supports_uidplus()`.
- [x] src/imap/backend.rs: `MoveStrategy` chosen from the two capabilities; `move_messages` runs `UID MOVE`, or `UID COPY` + `\Deleted` + `UID EXPUNGE`, or refuses.
- [x] Tests: the strategy for every capability combination.
- [x] Manual check against a server without MOVE (OVH Dovecot): `message move` and `message delete` succeed, and no other `\Deleted` message in the source is expunged.
- [x] CHANGELOG entry.
- [x] Fold the delta into [cairn/spec/backends.md](../../spec/backends.md); write the log entry.

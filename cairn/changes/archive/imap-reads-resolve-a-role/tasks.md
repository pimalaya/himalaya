---
cairn: tasks
change: imap-reads-resolve-a-role
---

# Tasks

- [x] src/imap/backend.rs: `get_message` resolves first; src/shared/client.rs says each adapter resolves.
- [x] Test `reading_resolves_a_mailbox_role` on the loopback server, which now lists `Sent Messages` with `\\Sent` (`a_refused_status_is_an_unreadable_mailbox` counts it); failing on e440b4e4 (`SELECT sent`).
- [x] Fold into cairn/spec/backends.md; CHANGELOG.

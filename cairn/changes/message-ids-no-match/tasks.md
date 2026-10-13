---
cairn: tasks
change: message-ids-no-match
---

- [x] `store_flags` returns a count on every backend and in `EmailClient`
- [x] IMAP: count from the `STORE` echoes or a `UID SEARCH UID <set>`
- [x] `flag add`, `flag set`, `flag remove`: fail on zero, report `count`, end the line
- [x] `message move`, `message copy`, `message delete` (to the trash): fail on zero
- [x] Tests: flag outputs in text and JSON
- [x] Manual check against Gmail over IMAP
- [x] Fold into spec/commands.md and spec/backends.md, log, CHANGELOG

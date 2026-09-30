---
cairn: tasks
change: imap-append-without-uid
---

# Tasks

- [x] IMAP and JMAP `add_message` return an optional id; Maildir, m2dir and pimdir keep a plain id, wrapped by `EmailClient::add_message`.
- [x] Log at debug level why an appended UID is unknown.
- [x] `message add` renders a missing id in text and as `null` in JSON.
- [x] Manual provider test (contributor, QQ Exmail): save-then-send goes on when the UID cannot be recovered.
- [x] The CHANGELOG entry.
- [x] Fold the delta into [cairn/spec/backends.md](../../spec/backends.md); write [cairn/log/2026-09-30-imap-append-without-uid.md](../../log/2026-09-30-imap-append-without-uid.md).

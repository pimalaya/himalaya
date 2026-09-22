---
cairn: tasks
change: imap-append-without-uid
---

- [x] Make the shared added-message id optional
- [x] Treat a successful IMAP append with no recoverable UID as success
- [x] Render and serialize an unknown added-message id honestly
- [x] Add regression coverage for an absent recovered UID
- [x] Update the current spec, changelog and dated log
- [x] Run formatting, the reduced IMAP plus SMTP tests, and the all-feature tests

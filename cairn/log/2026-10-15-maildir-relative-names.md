---
cairn: log
change: maildir-relative-names
landed: 2026-10-15
---

# Maildir mailboxes are named relative to the root

Raised in himalaya-tui#22. A Maildir's name was its last path segment (io-maildir `Maildir::name`), so `Archive` and `[Gmail]/Archive` both listed as `Archive`, and Maildir++ folders kept their leading dot. `MaildirClient::mailbox_name` in src/maildir/client.rs now names a Maildir through `MaildirStore::relative`, both layouts giving `[Gmail]/Archive`, the root keeping its folder name. `mailbox list` and `maildir list` use it, and `MaildirRow` lost its `From<Maildir>`, its name needing the store. Ids stay absolute paths.

Folded into backends (A Maildir is named relative to the root).

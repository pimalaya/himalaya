---
cairn: tasks
change: imap-special-use-aliases
---

- [ ] Land the extended-LIST RETURN option upstream in imap-codec (duesee/imap-codec#350)
- [ ] Expose LIST `RETURN (SPECIAL-USE)` on io-imap's LIST coroutine
- [ ] List mailboxes over the reused IMAP test connection in the wizard
- [ ] Map the RFC 6154 special-use attributes to alias keys, on top of `INBOX`
- [ ] Keep the fallback to `INBOX` only when the listing is empty or fails
- [ ] Update config.sample.toml and CHANGELOG

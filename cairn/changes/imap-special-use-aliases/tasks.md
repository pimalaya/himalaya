---
cairn: tasks
change: imap-special-use-aliases
---

- [ ] Land LIST-EXTENDED upstream in imap-codec (duesee/imap-codec#716, #718, #719, then the codec PRs)
- [ ] Expose LIST `RETURN (SPECIAL-USE)` on io-imap's LIST coroutine
- [ ] List with it in the IMAP backend and map the attributes onto `Mailbox.role`
- [ ] Answer `EmailClient::role_mailbox_id` from that listing for IMAP
- [ ] Update CHANGELOG

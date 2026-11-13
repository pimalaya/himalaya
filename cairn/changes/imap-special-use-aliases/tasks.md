---
cairn: tasks
change: imap-special-use-aliases
---

- [x] Map the special-use attributes of a plain LIST onto `Mailbox.role` (2026-11-12, the-live-campaign-fixes)
- [x] Answer `EmailClient::role_mailbox_id` and role words from that listing for IMAP (2026-11-12)
- [ ] Land LIST-EXTENDED upstream in imap-codec (duesee/imap-codec#716, #718, #719, then the codec PRs)
- [ ] Expose LIST `RETURN (SPECIAL-USE)` on io-imap's LIST coroutine
- [ ] List with it in the IMAP backend
- [ ] Update CHANGELOG

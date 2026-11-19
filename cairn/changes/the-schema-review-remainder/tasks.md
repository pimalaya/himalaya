---
cairn: tasks
change: the-schema-review-remainder
---

# Tasks

- [x] Owner: HIM-18 schema before v3 (wait for an io-imap release handing bytes) or after (v4) (2026-11-19: before v3, io-imap fixed first)
- [x] Owner: B5, B6, C2, C6 after v3 as additive, or now (2026-11-19: after v3)
- [ ] Adopt the io-imap release handing the raw reply as bytes, then land HIM-18's base64 shape in `imap raw`, its schema, spec, CHANGELOG and MIGRATION, before the bump
- [ ] After v3: B5 `ids` on `message copy|move`
- [ ] After v3: B6 once io-smtp returns the final reply; C2 once pimalaya-cli sets the rule; C6 once io-imap spells a response code (cross-repo)

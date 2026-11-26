---
cairn: tasks
change: the-schema-review-remainder
---

# Tasks

- [x] Owner: HIM-18 schema before v3 (wait for an io-imap release handing bytes) or after (v4) (2026-11-19: before v3, io-imap fixed first)
- [x] Owner: B5, B6, C2, C6 after v3 as additive, or now (2026-11-19: after v3)
- [x] Adopt the io-imap release handing the raw reply as bytes, then land HIM-18's base64 shape in `imap raw`, its schema, spec, CHANGELOG and MIGRATION, before the bump (2026-11-19: io-imap 0.12, an `undecoded` event `{base64}`, the-releases-of-2026-11-19)
- [ ] After v3: B5 `ids` on `message copy|move`
- [ ] After v3: B6 once io-smtp returns the final reply; C2 once pimalaya-cli sets the rule; C6 once io-imap spells a response code (cross-repo; io-imap 0.12 `ImapStatusError::code_bytes` spells it since 2026-11-19, C6 left after v3)
- [ ] Owner, before the tag: R1 to R5 of the review of 2026-11-26 (breaking), each fixed now or accepted for v3.
- [ ] After v3: R6 to R10 (not breaking), with a test that every key and every alternative carries a description.

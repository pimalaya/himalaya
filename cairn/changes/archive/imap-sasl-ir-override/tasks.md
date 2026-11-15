---
cairn: tasks
change: imap-sasl-ir-override
---

# Tasks

- [x] io-imap: add the connect options struct carrying starttls, auto-ID and the SASL-IR override; honour the override in connect instead of reading the capability unconditionally.
- [x] src/config.rs: add the `imap.sasl-ir` field.
- [x] src/imap/client.rs, src/account/check.rs and src/wizard/imap_smtp.rs: pass the override through the new connect options.
- [x] Document the option in config.sample.toml and both changelogs.
- [x] Build/test/fmt.
- [x] Fold into cairn/spec/provider-quirks.md (cairn/spec/config.md holds no per-backend keys, so nothing to fold there); log; land.

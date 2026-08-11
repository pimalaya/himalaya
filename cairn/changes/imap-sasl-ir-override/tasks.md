---
cairn: tasks
change: imap-sasl-ir-override
---

# Tasks

- [x] io-imap: add `ImapClientStdConnectOptions` with `starttls`, `auto_id` and
      `sasl_ir`; honour the override in `connect` instead of reading the
      capability unconditionally.
- [x] `config.rs`: add `ImapConfig::sasl_ir: Option<bool>`.
- [x] `imap/client.rs`, `account/check.rs` and `wizard/imap_smtp.rs`: pass the
      override through the new connect options.
- [x] Document the option in `config.sample.toml` and both changelogs.
- [x] Build/test/fmt.
- [x] Fold into `cairn/spec/provider-quirks.md` (config.md holds no per-backend
      keys, so nothing to fold there); log; land.

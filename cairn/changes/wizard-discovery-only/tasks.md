---
cairn: tasks
change: wizard-discovery-only
---

- [x] Route every non-folder input (email, bare domain, `scheme://` URL) through the discovery flow; keep the folder path on the local backend
- [x] For a URL input, discover from its host and filter the discovered entries by scheme (`imap`/`imaps` → IMAP+SMTP with the TLS constraint for `imaps`; `jmap`/`jmaps`/`http`/`https` → JMAP), dropping proprietary entries
- [x] Stop the wizard with the can't-discover message + config-sample link when discovery yields no supported config (empty result, deadline elapsed, or scheme filter emptied)
- [x] Extract the config-sample URL into a shared `const`, reused by the welcome banner and the stop message
- [x] Remove `manual_fallback` and `split_email` from `discover.rs`
- [x] Remove `configure_server`'s hand-entry routing
- [x] Remove `imap_smtp::configure_manual` and its now-unused helpers (`prompt_smtp_endpoint`, unused seed/probe helpers)
- [x] Remove `imap_smtp::default_smtp`; when IMAP is discovered but SMTP is not, produce an IMAP-only account (no `smtp` block, no SMTP test)
- [x] Remove `jmap::configure_manual`
- [x] Relocate the `command_secret`/`shell_secret` helpers into `wizard/secret.rs` (with their tests) and delete the now-dead `wizard/account.rs` converters + `pub mod account`
- [x] Auto-detect the local backend from markers (`.m2store`/`.m2dir` → m2dir, `cur`/`new`/`tmp` → Maildir); prompt only when compiled with both and detection is inconclusive
- [x] Fix module/function doc comments that describe a manual fallback
- [x] Fold the delta into `cairn/spec/wizard.md`, update CHANGELOG, and add the log entry
- [x] `cargo fmt` + build + test the affected feature combinations

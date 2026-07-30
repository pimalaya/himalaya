---
cairn: log
change: wizard-discovery-only
landed: 2026-07-30
---

# Wizard is discovery-only

Made the configuration wizard configure an account only from what it can discover automatically, and deleted every hand-entry path. An email, a bare domain and a `scheme://` server URL now all run discovery through a single `configure_discovery`: a URL discovers from its host, and its scheme narrows the results — `imap`/`imaps` keep IMAP + SMTP (with `imaps` requiring an implicit-TLS IMAP endpoint), the HTTP-family schemes keep JMAP, and proprietary Gmail/Graph entries are dropped when a scheme is named. When discovery yields nothing (empty result, deadline elapsed, or scheme filter emptied) the wizard stops with a message pointing at the documented sample config, rather than prompting for fields.

Removed the hand-entry surface: `manual_fallback`, `configure_server` and `split_email` from `wizard/discover.rs`, `configure_manual` + `prompt_smtp_endpoint` + `default_smtp` + the `seed_imap`/`wizard_imap_server` helpers from `wizard/imap_smtp.rs`, and `configure_manual` from `wizard/jmap.rs`. That left `wizard/account.rs` (the `WizardImapConfig`/`WizardSmtpConfig` → config converters) dead except for two `pub(crate)` secret builders shared with `wizard/secret.rs`; those two moved into `secret.rs` (with their rejection tests) and `account.rs` was deleted.

The wizard no longer invents an SMTP host. IMAP was already never guessed — no discovered IMAP means no entry — but SMTP fell back to `smtp.<domain>` and then failed its own connection test when that host did not exist (the #722 shape). Now `configure_discovered` returns an optional SMTP config, and `Chosen::ImapSmtp` carries `Option<Box<SmtpConfig>>`: discovery with IMAP but no submission endpoint yields an IMAP-only account, no SMTP block and no SMTP test.

The local folder path now auto-detects the store kind from on-disk markers (`.m2store`/`.m2dir` → m2dir, `cur`/`new`/`tmp` → Maildir) and only prompts Maildir-vs-m2dir when both backends are compiled in and the directory is ambiguous.

Moved the `wizard` capability: MODIFIED "Input orients the flow", "Discovery is time-bounded", "One entry per service, then auth" and "Per-protocol test and shared SMTP credentials"; ADDED "Stop when nothing is discovered" and "Local backend auto-detected".

Verified: default build and all single-backend feature combinations compile, `cargo fmt`/`clippy` clean, and the wizard unit tests pass (including the two secret-builder tests that moved into `secret.rs`).

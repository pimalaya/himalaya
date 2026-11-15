---
cairn: tasks
change: the-family-releases-adopted
---

# Tasks

- [x] Cargo.toml: io-oauth 0.6 (dev), io-gmail 0.7, io-imap 0.9, io-jmap 0.8, io-maildir 0.4, io-mbox 0.2, io-managesieve 0.3, io-msgraph 0.8, io-sasl 0.2, io-smtp 0.8; six git pins removed; `cargo update` for those crates and io-http; one io-http, one io-sasl and one io-oauth in `cargo tree`.
- [x] SASL-S1: `cargo tree -e features -i io-imap` and `-i io-smtp` showing no `scram`; `imap` enables `io-imap/scram`, `smtp` enables `io-smtp/scram`.
- [x] SMTP-S1: `SmtpMailbox::try_from` in src/smtp/backend.rs and src/smtp/send.rs, the `envid` result propagated; control characters refused in src/email/submission.rs, `an_address_holding_a_control_character_is_refused` seen failing first.
- [x] IMAP: `output_data` (src/imap/utils.rs) at every command returning an `ImapOutput`; `get_message` on a skipped line; `COPYUID` pairs in `imap copy`, `imap move`, `copy_messages` and `move_messages`; `ImapStatusError` in the outcome; session options.
- [x] mbox: `update` planning its edits from the index and starting over once on `Stale`, `a_write_rebuilds_a_stale_index_once` seen failing first; `MboxKeyword`; `resolve_name`.
- [x] JMAP: raw method responses in `jmap query`; `Option<Option<T>>` in `jmap mailbox update`; the warning of a refused implicit `Email/set`.
- [x] Gmail `Unknown` variants; Graph `user_path`, `message_get` and `MsgraphField` recipients; ManageSieve and SMTP session options.
- [x] fmt; clippy over the fourteen feature sets of the workflow; `cargo test --all-features`; `cargo test --no-default-features --features rustls-ring,maildir`.
- [x] CHANGELOG; fold the delta into the spec (backends, commands, packaging); log; status landed.

---
cairn: change
id: the-config-splits-by-block
status: active
created: 2026-11-14
---

# The config splits by block

## Why

src/config.rs holds the whole schema in 1,726 lines and carries 61 lint and feature attributes: 17 `#[cfg(..)]` (imports, `parse_server`, the TLS, proxy and SASL conversions, six tests), 23 `cfg_attr(not(..), allow(dead_code))` and 21 `#[allow(unused)]`. The configuration a build parses is already the same in every build (every-feature-set-builds-clean), yet the module reads as conditional, and each new option has to pick the right feature list (#809 added one more).

The gates come from runtime code living beside the schema: the conversions into pimalaya-stream and io-sasl values, the server URL parsing, the no-backend error. cairn/spec/config.md already calls the schema pure DTOs; the code does not match.

## What

- The configuration is a plain model: its types, their serde attributes and defaults, and the loading contract (`TomlConfig`, `AccountConfig::inherit_proxy`). It holds no feature gate and depends on serde and pimalaya-config alone, so it parses the same with any feature on or off.
- The features turn it into runtime values. The conversions move out as inherent impls on the config types, so call sites do not change, each a root module gated once in src/main.rs with exactly the features using it and holding no gate inside:
  - src/tls.rs: `TlsConfig::into_tls`, with the six network backends
  - src/proxy.rs: `ProxyConfig::resolve` and `ProxyConfig::try_into_proxy`, with the six network backends
  - src/sasl.rs: `SaslConfig::try_into_sasl`, with IMAP, SMTP and ManageSieve
  - src/server.rs: `parse_server` and its tests, with IMAP, SMTP, JMAP and ManageSieve
- `AccountConfig::no_backend_error` moves to src/backend.rs beside `Backend`, ungated as today.
- One lint attribute at the configuration root replaces the 44 per-item allows, a field being read only by the backends using it: `#![cfg_attr(not(all(feature = "imap", feature = "smtp", feature = "jmap", feature = "gmail", feature = "msgraph", feature = "maildir", feature = "mbox", feature = "sieve", feature = "pimdir")), allow(dead_code))]`. The default and `--all-features` builds still report a field parsed and never read.
- The schema splits by block under src/config/, one file per TOML block, src/config.rs keeping its own code (module header, `Config`, `AccountConfig`):
  - src/config/imap.rs: `ImapConfig`, `ImapSortConfig`, `ImapIdConfig`
  - src/config/smtp.rs: `SmtpConfig`
  - src/config/sieve.rs: `SieveConfig`
  - src/config/jmap.rs: `JmapConfig`, `JmapAuthConfig`
  - src/config/gmail.rs: `GmailConfig`, `GmailAuthConfig`
  - src/config/msgraph.rs: `MsgraphConfig`, `MsgraphAuthConfig`
  - src/config/maildir.rs: `MaildirConfig`, `MaildirKeywordsConfig`, `MaildirKeywordHeaderConfig`, `MaildirLayoutConfig`
  - src/config/mbox.rs: `MboxConfig`, `MboxFormatConfig`, `MboxLockConfig`
  - src/config/pimdir.rs: `PimdirConfig`
  - src/config/tls.rs: `TlsConfig`, `TlsClientConfig`, `TlsProviderConfig`, `RustlsConfig`, `RustlsCryptoConfig`
  - src/config/proxy.rs: `ProxyConfig`
  - src/config/sasl.rs: `SaslConfig` and its six mechanism types
  - src/config/envelope.rs: `EnvelopeConfig`, `EnvelopeListConfig`
  - src/config/mailbox.rs: `MailboxConfig`
  - src/config/message.rs: `MessageConfig`, `MessageQuoteConfig`, `PostingStyle`, `MessageSendConfig`, `SaveCopyConfig`
- Each block's defaults (`default_imap_alpn` and the like) move with it and become private, nothing outside the module reading them. Each test moves with the block it parses; the tests over the whole file (the sample, account resolution, proxy inheritance) stay in src/config.rs.
- Type names stay (`ImapConfig`, not `imap::Config`); callers import from the new paths, no re-export.
- Spec: cairn/spec/backends.md makes the configuration module a model holding no feature gate, its conversions living with the features using them.

## Scope / non-goals

No behaviour change: every build parses, accepts and refuses exactly what it does today, and every output stays. The sample shape (config.sample.toml) is untouched. A field the default build reports unread once the allows are gone is a finding to report, not something to allow back.

## Delivery

Two commits, each asked for before it is made: `refactor(config): move the conversions to the features using them` (the four root files, `no_backend_error`, the attributes dropped), then `refactor(config): split the schema by block` (moves and import paths, no line of logic changed).

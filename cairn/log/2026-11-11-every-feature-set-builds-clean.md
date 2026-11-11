---
cairn: log
change: every-feature-set-builds-clean
landed: 2026-11-11
---

# Every feature set builds clean

`cargo clippy --all-targets -- -D warnings` passes over the whole feature matrix, where only the default features, `--all-features` and `rustls-ring,imap,smtp` passed. Before, as error lines of the binary target (the tests were not reached where it failed): `rustls-ring` alone 111, ManageSieve alone 88, Graph alone 34, each local backend alone and the four together 20, JMAP alone, Gmail alone and `native-tls,jmap` 14, SMTP alone 2, IMAP alone none.

- src/config.rs: `anyhow::{Result, bail}` imported with the network backends. Every build parses the whole schema, so the types only some backends read stay compiled and take `cfg_attr(not(..), allow(dead_code))` with exactly those backends, the reason given once in the module header: `ProxyConfig`, `TlsConfig` and `RustlsConfig` (IMAP, SMTP, JMAP, Gmail, Graph, ManageSieve), `SaslConfig` and its six mechanism types (IMAP, SMTP, ManageSieve), `ImapSortConfig` and `ImapIdConfig` (IMAP), `SaveCopyConfig` (the shared tree), and `Config`, `AccountConfig` and the envelope, mailbox and message types the account context reads (any client). A `cfg` there would have made a build accept a block a full build refuses: a Maildir-only build still refuses an unknown `imap.sort` key, checked on a scratch configuration with `account list`. The `#[allow(unused)]` on the per-backend blocks is unchanged.
- src/account: `context` compiled with any client (`any(backend, feature = "smtp", feature = "sieve")`); in it, the accessors move to their own `impl Account` block, compiled with the shared tree (`any(backend, feature = "smtp")`) as are the seven defaults, the imports only they use and the tests, `merge` and the two `From` staying wherever the module is. ManageSieve builds its account and reads none of it.
- src/session.rs: `SessionAccount::run`, its imports and its test with any client. src/main.rs: the session literal allows `clippy::needless_update` in a build without any client, `account` being its only field there.
- src/main.rs `mod email` and src/shared/mod.rs `message` with the shared tree, `crlf` with any client (`any(backend, feature = "smtp", feature = "sieve")`); the inner gates of src/shared/message/mod.rs and `email::vacation` they made redundant go, the message module doc saying where `arg` serves.
- src/email/search: the sort evaluation (`sort_envelopes`, `compare_with`, `first_addr_key`, and its test) moves to src/email/search/sort/eval.rs, compiled with the local backends and Graph; src/email/search/eval.rs keeps the filter evaluation, compiled with the local backends alone, as are `address::from_mime`, `part::has_attachment` and `part_has_attachment`, Graph dropped from their gates and from `marks_attachment` and `is_protocol_part`. src/maildir, src/mbox, src/m2dir, src/pimdir and src/msgraph backend.rs import `sort::eval::sort_envelopes`. The eval tests import through the crate path rather than `super::*`.
- src/shared/receipt.rs: `IdsReceipt` and its `Display` with Gmail, JMAP, Maildir, mbox and m2dir, the backends printing it; src/json_schema.rs imports it with them and `pimalaya_cli::output::{Bytes, Paginated}` with the shared tree. src/mcp.rs: `paths_closed` with the shared tree, the test imports used by the shared-tree tests alone with them.

No behaviour moved: the commands, outputs and accepted configurations of every build are the same.

Capabilities moved: backends (cairn/spec/backends.md: "Every backend builds alone" added; it belongs beside "Feature-gated backends" in cairn/spec/packaging.md, which the live-provider-tests change edits today).

Verified: rustfmt over src/; clippy `--all-targets -D warnings`, all clean, with the default features, `--all-features`, `--no-default-features` plus `rustls-ring`, `rustls-ring,imap`, `rustls-ring,smtp`, `rustls-ring,jmap`, `rustls-ring,gmail`, `rustls-ring,msgraph`, `rustls-ring,maildir`, `rustls-ring,mbox`, `rustls-ring,m2dir`, `rustls-ring,sieve`, `rustls-ring,pimdir`, `native-tls,jmap` and `rustls-ring,maildir,mbox,m2dir,pimdir`; `cargo test --all-features` passes (429 unit tests, 1 ignored, and the integration suites, the live ones ignored).

Left: `cargo test --no-default-features --features rustls-ring,maildir` fails two tests of tests/mcp.rs, `write_and_send_tools_under_their_flags` and `a_tool_the_account_cannot_serve_is_not_listed`, which expect `message_send` listed where no SMTP is compiled: they want `#[cfg(feature = "smtp")]`, tests/ being held by the live-provider-tests change today. Its 216 unit tests and every other suite pass. CI checks none of the matrix yet.

---
cairn: tasks
change: every-feature-set-builds-clean
---

# Tasks

- [x] Baseline: clippy `--all-targets -D warnings` over the default features, `--all-features`, `rustls-ring` alone, each backend alone, `native-tls,jmap` and the four local backends, diagnostics listed per set
- [x] src/config.rs: `anyhow::{Result, bail}` with the network backends; `cfg_attr(not(..), allow(dead_code))` on the proxy, TLS and rustls types (network backends), the SASL types (IMAP, SMTP, ManageSieve), the IMAP `sort` and `id` types (IMAP), `SaveCopyConfig` (the shared tree) and the account-wide types (any client); the reason in the module header
- [x] src/account/context.rs: the module with any client; the accessors, defaults and their imports and tests with the shared tree
- [x] src/session.rs: `SessionAccount::run`, its imports and test with any client; src/main.rs: `clippy::needless_update` allowed in a build without client
- [x] src/main.rs `email` and src/shared/mod.rs `message` and `crlf` gated once at the parent, the redundant inner gates dropped
- [x] src/email/search: the sort evaluation to `sort::eval` (local backends and Graph), the filter evaluation, `address::from_mime` and the parsed attachment mark with the local backends alone; the five callers moved
- [x] `IdsReceipt` and its schema import with the backends printing it; `mcp::paths_closed` and the MCP test imports with the shared tree; `pimalaya_cli::output` in src/json_schema.rs with the shared tree
- [x] Clippy clean over the whole matrix; `cargo test --all-features`; rustfmt
- [x] tests/mcp.rs: gate `write_and_send_tools_under_their_flags` and `a_tool_the_account_cannot_serve_is_not_listed` on `feature = "smtp"`, both expecting `message_send` listed, which a build without SMTP cannot serve; `cargo test --no-default-features --features rustls-ring,maildir` passes
- [x] A Maildir-only build refusing an unknown `imap.sort` key as a full build does (the delta's second scenario), run on a scratch config
- [x] Fold the delta into cairn/spec/backends.md; log; status landed

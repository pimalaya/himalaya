---
cairn: change
id: every-feature-set-builds-clean
status: landed
created: 2026-11-11
---

# Every feature set builds clean

## Why

`cargo clippy --all-targets -- -D warnings` passes with the default features, `--all-features` and `rustls-ring,imap,smtp`, the three sets checked so far, and fails on dead code and unused imports elsewhere: with `rustls-ring` alone, `native-tls,jmap` and the four local backends together, which dependencies-trimmed (2026-11-11) left open, and with each backend alone but IMAP. Counted as error lines of the binary target: 111 with `rustls-ring` alone, 88 with ManageSieve, 34 with Graph, 20 with each local backend and with the four together, 14 with JMAP, Gmail and `native-tls,jmap`, 2 with SMTP. The causes are few. The configuration types of the network backends (proxy, TLS, SASL, the IMAP `sort` and `id` options) are read by their backends alone, yet compiled in every build; the merged account context, the shared email types, the search grammar and the message input are compiled where no command reads them; Graph compiles the local filter evaluation for the one sort it needs; a few imports and helpers (`IdsReceipt`, `mcp::paths_closed`, `crlf::normalize`) outlive their users. Nothing checks these builds, so they rot between releases, and a distribution packaging one backend meets them first.

## What

- Gate on the items, `#[cfg]` with the features that use them: the `email` module, `shared::message` and `account::context` with the builds carrying a mail transport (`any(backend, feature = "smtp")`, ManageSieve added for the account context, `SessionAccount::run` and `shared::crlf`), the context's accessors and defaults with the shared tree, `IdsReceipt` with the backends printing it, `mcp::paths_closed` with the message input, the imports and test modules with their users.
- Split the search evaluation: the sort moves to `email::search::sort::eval`, compiled with the local backends and Graph; the filter stays in `email::search::eval`, compiled with the local backends alone, and so do `address::from_mime` and the attachment mark of a parsed message.
- Keep the configuration schema whole in every build: every build parses every block, a compiled-out backend's included, so a type only some backends read takes `cfg_attr(not(..), allow(dead_code))` with exactly those backends rather than a `cfg`, which would change what a build accepts. The existing `#[allow(unused)]` on the per-backend blocks stays.
- One clippy statement allow: `Session { account, ..Default::default() }` has no other field in a build without any client (`clippy::needless_update`), allowed there alone.
- Spec: the build matrix as a requirement of cairn/spec/backends.md (cairn/spec/packaging.md, the natural home beside "Feature-gated backends", is being edited by the live-provider-tests change at the same time).

## Scope / non-goals

No behaviour changes: every command, output and configuration a build accepted it still accepts. The CI wiring of the matrix is left to the workflow owner; the change lists the sets to check.

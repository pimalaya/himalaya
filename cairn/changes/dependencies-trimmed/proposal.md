---
cairn: change
id: dependencies-trimmed
status: landed
created: 2026-11-11
---

# Dependencies trimmed

## Why

Ortie trimmed its dependencies on 2026-11-09 (ortie 2026-11-09-dependencies-trimmed, 2026-11-09-deny-warnings-cleared): every dependency names its features, url's IDNA runs without its Unicode back end, base64 gave way to base64-codec, and `cargo deny check` runs over the release targets. Himalaya has not followed. Most of its dependencies take their upstream defaults, so a default turned on upstream reaches the binary unasked; open, convert_case and uds_windows are declared and never used (shellexpand left with the wizard); base64 0.23 is a direct dependency beside the base64-codec mime-rs already brings; url's IDNA compiles the 21 ICU4X crates, which only a URL host written in Unicode needs; deny.toml checks every target, Android's included, and allows licences no crate carries any more. Two stale points sit beside it: the IMAP envelope decodes its RFC 2047 words through rfc2047-decoder, which brings a second base64 and reads a header unlike `message read` does, and `JmapConfig::server` documents `jmap` and `jmaps` schemes the code refuses.

## What

- Cargo.toml: every dependency, build and dev ones included, declares `default-features = false` and the features Himalaya uses; open, convert_case and uds_windows go; toml moves to the dev-dependencies, the tests being its only user; pimalaya-cli's `prompt` moves under the `pimdir` feature, `pimdir queue cancel` being its only user.
- base64 to base64-codec 0.1 in Himalaya's own code: the `imap fetch` body, the JMAP `Basic` header, the `attachment download --stdout` data and the MCP `attach-data` decoding, byte for byte as before, each covered by a test. `main` installs the detected CPU token first (simd-cpu 0.1), so the codecs mime-rs runs use it too.
- `idna_adapter = "=1.0.0"`: a non-ASCII host in an `http` or `https` URL (a JMAP server, a proxy) is refused when parsed, its Punycode form (`xn--…`) working. IMAP, SMTP and ManageSieve servers are URLs of schemes url treats as opaque, which never reach IDNA: a non-ASCII host there was percent-encoded and never resolved, before as after.
- The IMAP envelope subject and names decode through the shared header reading `message read` uses (src/shared/message/header.rs), rfc2047-decoder going with base64 0.22, charset and quoted_printable.
- deny.toml: Ortie's `[graph] targets`; the licences nothing needs any more leave the allowed list; `cargo deny check` reports only the duplicates the tree cannot avoid.
- `JmapConfig::server`'s doc names the `http` and `https` schemes the code takes.
- CHANGELOG; spec/packaging.md; log.

## Scope / non-goals

The git and path pins of `[patch.crates-io]` stay as they are (io-http, io-jmap, io-msgraph, io-pimdir by rev; pimalaya-cli and pimalaya-config by path). Duplicates another crate causes (getrandom 0.2 under ring, syn 2 under bounded-static-derive and windows-core, hashbrown 0.15 under chumsky) stay visible warnings. The `pimdir queue cancel` confirmation keeps inquire.

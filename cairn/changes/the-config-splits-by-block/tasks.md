---
cairn: tasks
change: the-config-splits-by-block
---

# Tasks

- [ ] src/tls.rs, src/proxy.rs, src/sasl.rs, src/server.rs: the conversions and `parse_server` moved as inherent impls and a free parser, with their imports and tests; each gated once in src/main.rs
- [ ] src/backend.rs: `AccountConfig::no_backend_error` moved beside `Backend`
- [ ] src/config.rs: drop the 17 feature `#[cfg]`, the 23 `cfg_attr(.., allow(dead_code))` and the 21 `#[allow(unused)]`; add the root `cfg_attr(not(all(..)), allow(dead_code))`; rewrite the module header (a plain model, the features converting it); no import left from pimalaya-stream, io-sasl, secrecy or url
- [ ] Build with the default features and `--all-features`: list every field reported unread, report each (real unwired option or test-only read) before changing anything
- [ ] Clippy `--all-targets -D warnings` over the 14 feature sets of tests.yml; `cargo test --all-features`; `cargo test --no-default-features --features rustls-ring,maildir` (the sample test parses every section in a Maildir-only build)
- [ ] Commit one, after asking
- [ ] src/config/: the fifteen block files, moved verbatim with their docs and tests; `mod` declarations after the src/config.rs header
- [ ] Callers: import paths from `crate::config::<block>::*Config`, no re-export
- [ ] Same checks as commit one, plus `cargo fmt --check`
- [ ] Commit two, after asking
- [ ] Fold the delta into cairn/spec/backends.md; log entry; status landed

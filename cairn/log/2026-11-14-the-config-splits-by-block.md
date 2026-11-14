---
cairn: log
change: the-config-splits-by-block
landed: 2026-11-14
---

# The config splits by block

src/config.rs held the whole schema in 1,726 lines with 61 feature and lint attributes, the TLS, proxy and SASL conversions and the server parsing living beside the types. The configuration is now a plain model with no feature gate: the conversions moved to src/tls.rs, src/proxy.rs, src/sasl.rs and src/server.rs as inherent impls, each gated once in src/main.rs with the features using it, and `no_backend_error` to src/backend.rs. One root `cfg_attr(not(all(..)), allow(dead_code))` replaces the per-item allows, so the default and `--all-features` builds still report a field parsed and never read; none was. The schema then split into fifteen block files under src/config/, src/config.rs keeping `Config` and `AccountConfig`. No behaviour changed: clippy passes over the 14 feature sets of tests.yml, and both test runs keep their counts (568 with every feature, 267 with Maildir alone).

Spec: backends ("Every backend builds alone", the configuration rule rewritten).

---
cairn: delta
change: the-config-splits-by-block
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Every backend builds alone
(backends) `cargo clippy --all-targets -- -D warnings` SHALL pass with the default features; with `--all-features`; with `--no-default-features` and a TLS provider alone (`rustls-ring`); with a TLS provider and each backend feature alone (`imap`, `smtp`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox`, `sieve`, `pimdir`); with `native-tls,jmap`; and with the three local backends together (`maildir,mbox,pimdir`). Code only some backends use SHALL be gated on the item with exactly those features, never allowed crate-wide. The configuration SHALL be a model holding no feature gate and no runtime code, compiled in every build, so every build parses and checks the same configuration, the blocks of the backends it leaves out included; it SHALL allow dead code once, at its root, in the builds missing a backend. Turning it into runtime values (TLS, proxy, SASL, server URL) SHALL belong to the features using them, gated with exactly those.

## REMOVED Requirements

None.

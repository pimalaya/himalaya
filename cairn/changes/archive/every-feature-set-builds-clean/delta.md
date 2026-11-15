---
cairn: delta
change: every-feature-set-builds-clean
---

# Delta

## ADDED Requirements

### Requirement: Every backend builds alone
(backends) `cargo clippy --all-targets -- -D warnings` SHALL pass with the default features; with `--all-features`; with `--no-default-features` and a TLS provider alone (`rustls-ring`); with a TLS provider and each backend feature alone (`imap`, `smtp`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox`, `m2dir`, `sieve`, `pimdir`); with `native-tls,jmap`; and with the four local backends together (`maildir,mbox,m2dir,pimdir`). Code only some backends use SHALL be gated on the item with exactly those features, never allowed crate-wide. A configuration type only some backends read SHALL stay compiled in every build and be allowed to go unread in the builds without them, so every build parses and checks the same configuration, the blocks of the backends it leaves out included.

#### Scenario: A Maildir-only build
- GIVEN `--no-default-features --features rustls-ring,maildir`
- WHEN `cargo clippy --all-targets -- -D warnings` runs
- THEN it passes, where it reported the proxy, TLS, SASL and IMAP options of the configuration unread

#### Scenario: A block for a compiled-out backend
- GIVEN a Maildir-only build and an account whose `imap.sort` block carries an unknown key
- WHEN `account list` reads the configuration
- THEN it is refused naming that key, as a full build refuses it

## MODIFIED Requirements

None.

## REMOVED Requirements

None.

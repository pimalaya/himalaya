---
cairn: log
change: message-read-invalid-recipients
date: 2026-10-06
---

# The read view lists the address entries it cannot read

The `message read` view dropped an address entry it could not read as an address, so a client confirming a draft before sending it (MOA's send preview) could not tell that a recipient would be lost. `headers.invalid` now lists those entries, header by header.

## What landed

- src/shared/message/read.rs: `InvalidAddresses` (`to`, `cc`, `bcc`, `replyTo`, `from`, `sender`) in `MessageHeaders`; the address parser returns the entries it drops beside the valid list, as decoded text in header order. What is left of a group name, an empty group or a trailing separator is no entry. `message parse` prints the same view through `MessageReadOutput`.
- Unit tests: a group holding an invalid entry, a bare word, `a@localhost`, a quoted local part, an encoded-word name with no address, an empty group and a lone comment, a header of valid entries only, the order of the entries.

Capabilities moved: **commands** (the read view lists unreadable address entries).

## Verification

`cargo test --all-features`: 225 unit tests and 6 integration tests passed. `cargo clippy --all-features --all-targets` clean, `cargo fmt --check` clean, `cargo build --no-default-features --features rustls-ring,imap,smtp,msgraph,pimdir` builds. The 13 messages of MOA's parity vectors print the same through `message read --json` on a Maildir and `message parse --json` with an empty `HOME`.

---
cairn: log
change: managesieve-support
landed: 2026-08-24
---

# ManageSieve support

The optional `sieve` feature adds an account-scoped ManageSieve transport and
protocol command family. It supports `sieve://`, `sieves://`, and Unix socket
servers, implicit TLS, STARTTLS, LOGIN and PLAIN authentication, capabilities,
script listing and retrieval, upload and validation, activation/deactivation,
deletion, and a diagnostic raw command.

The wire implementation keeps CRLF framing, quoted strings, literals, and
server status handling inside the Sieve client. The CLI follows the existing
protocol-module style and exposes structured output for capabilities, scripts,
and script retrieval.

Verification completed locally:

- `cargo fmt --all`
- `cargo test --all-targets` — 116 tests passed
- `cargo test --no-default-features --features sieve,rustls-ring` — 71 tests passed
- `cargo check --no-default-features --features sieve,rustls-ring`
- `cargo build --no-default-features --features imap,smtp,rustls-ring`
- `cargo build --no-default-features --features jmap,rustls-ring`
- `cargo deny check`
- Sieve-focused Clippy with the pre-existing wizard lint allowed
- release-feature build and CLI help smoke tests

The fake-server test covers the command sequence and literal handling without
touching a real mailbox.

Live read-only smoke test completed against the user's global `franz` account
(`franz@bett.ag`) on `mail2.anycast.io:4190`: implicit TLS was rejected by the
server, while `sieve://` plus STARTTLS succeeded. Dovecot Pigeonhole reported
Sieve 1.0 and SASL PLAIN; authenticated `LISTSCRIPTS` returned an empty list,
and `account check --backend sieve` returned `OK`. The server-side
`.dovecot.sieve` was a regular file outside the personal ManageSieve store.
With explicit authorization, Himalaya then uploaded the supplied rules as
`main` and activated it. `LISTSCRIPTS` showed `main` active and `dovecot.orig`
inactive; `GETSCRIPT` verified that both contained the original rules. No
script was deleted manually. A cleartext-auth guard bug found during the
first test was fixed so PLAIN/LOGIN are allowed after STARTTLS but remain
blocked on unencrypted connections.

The final local hardening also rejects overlong response lines and caps
server-declared literals before allocation; parser tests cover literal token
boundaries and the allocation guard.

The same authorized migration was then verified for `mail@anycast.io` on
`mail1.anycast.io`. The existing regular `.dovecot.sieve` contained the
invoice forwarding rule. Himalaya uploaded it as `main` and activated it;
Dovecot created the managed `sieve/main.sieve` symlink and preserved the old
file as inactive `dovecot.orig`. `sieve list`, `doveadm sieve list`, and
`doveadm sieve get` agreed on the active script and its contents. No manual
symlink or direct filesystem edit was used.

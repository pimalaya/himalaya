---
cairn: log
change: managesieve-extraction
landed: 2026-08-25
---

# ManageSieve moved into a library

The nine hundred lines of RFC 5804 that landed in src/sieve/protocol.rs and src/sieve/client.rs are gone from this repository. They are now io-managesieve, a new I/O-free library shaped like io-imap and io-smtp, and what remains here is the shape every other protocol module already had: a `SieveClient` wrapping `ManagesieveClientStd`, one file per subcommand, and Himalaya's own serializable output types.

The extraction was not a move. Almost none of the old code survived, because the layering it was written against was the wrong way round: a binary owning a lexer, a socket loop and a SASL exchange, none of which is Himalaya's. Rewriting it as coroutines put each of those where the other protocols keep them, and three things fell out of that on their own.

Authentication is the largest. The old client dispatched on two `Sasl` variants and reported the rest as unsupported, which meant a `[sieve]` block accepted less than the `[imap]` block above it in the same account. RFC 5804 frames every mechanism identically, a name and a base64 string each way, so one coroutine now serves all of them and the dispatch is a match on `io_sasl::mechanism::Sasl`. SCRAM-SHA-256, OAUTHBEARER and XOAUTH2 work here now because nothing had to be written for them. So does CRAM-MD5, which is server-first and which neither io-imap nor io-smtp carries: the ManageSieve framing needs no special case for a mechanism that speaks second, since the initial response is optional in the grammar rather than gated on a capability.

Response codes are the second. The old parser kept the completion line as text, so `NO (NONEXISTENT) "no such script"` and `NO (QUOTA/MAXSIZE) "too big"` reached the user as the same shape and a caller could act on neither. `ManagesieveResponseCode` parses the eleven the specification defines, keeps the name of anything it does not know, and the QUOTA hierarchy folds an unknown detail back onto its parent as the RFC asks. `sieve put` and `sieve check` now report the WARNINGS text a server attaches to a script that compiled but probably is not what its author meant.

The third is coverage of the specification itself. RENAMESCRIPT, NOOP and UNAUTHENTICATE were missing. All three are in the library; `sieve rename` is the one that reached the CLI, renaming being the operation a user actually performs and the emulation the RFC spells out for servers lacking it leaving a window with no active script.

Two guards from the old code were kept and moved somewhere they cannot be forgotten. Refusing to send a password over a cleartext connection now lives in the session coroutine rather than in a Himalaya `if`, since it is what RFC 5804 section 5 asks implementations to carry; `sieve.allow-cleartext-auth` opts out, for a server reached over a trusted local link. Refusing bytes that arrive past a greeting or past a STARTTLS reply is the injection defence io-imap and io-smtp already had, and here the coroutine fails rather than handing the bytes back, since a returned `Vec<u8>` is something a caller can ignore and there is no legitimate reason to continue.

One default changed on the way, and it is the only breaking bit. A bare `sieve.server` used to resolve to `sieves://`, matching the rule the `imap` and `smtp` blocks follow, and against ManageSieve that rule reaches nothing: RFC 5804 registers one port, 4190, and defines STARTTLS as the way to TLS on it, so an implicit-TLS default fails against every stock Dovecot. A bare authority now resolves to `sieve://`, and `sieve.starttls` left unset follows the scheme rather than defaulting to false. `sieves://` stays accepted, deployments listening for a TLS handshake straight away being a real if unregistered thing, and setting `starttls` on one is an error. Nothing is less encrypted for it: a session that cannot upgrade fails on the missing capability, and the credentials are refused in the clear regardless.

One deliberate wart. Himalaya depends on io-managesieve through a path, because the crate is not published yet. The `version = "0.1"` alongside it is what takes over the day it is, and until then this repository does not build outside a checkout that has io-managesieve as a sibling.

Verification: `cargo fmt`, `cargo clippy --all-targets` clean but for the pre-existing wizard lint, `cargo test` at 109 passing, and a smoke test of every subcommand against a scripted local ManageSieve server, covering the capability greeting, PLAIN with an inline initial response, a literal script name carrying a space, GETSCRIPT literals, PUTSCRIPT warnings, activation, renaming, a NO carrying the ACTIVE code, `sieve raw`, `--json` output, `account check` and `account list`. The cleartext refusal was checked by removing the flag and watching the connection stop before the credentials went out.

Capabilities moved: commands, config, packaging, testing.

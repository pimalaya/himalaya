---
cairn: change
id: managesieve-extraction
status: landed
created: 2026-08-25
---

# Move ManageSieve into a library

ManageSieve landed in Himalaya as src/sieve/protocol.rs plus src/sieve/client.rs: a hand-rolled lexer, a blocking socket loop, and a two-mechanism SASL exchange, roughly nine hundred lines living inside a binary. Nothing about that code is Himalaya's: the protocol is RFC 5804, the same one himalaya-tui, a mobile client or a future service would speak, and none of them can reach it where it sits.

It also sat outside the shape every other backend in this repository has. io-imap, io-smtp, io-jmap and the rest are I/O-free coroutine libraries with an optional std client, so Himalaya holds a thin `build_<proto>_client` and one file per subcommand. Sieve alone held its own transport, its own framing and its own authentication, which is the layering rule inverted: the binary owning protocol knowledge the libraries are supposed to carry.

The gap that inversion left is not theoretical. The in-repo client spoke LOGIN and PLAIN and reported everything else as unsupported, where the same account's IMAP and SMTP blocks already accept SCRAM-SHA-256, OAUTHBEARER and XOAUTH2 through io-sasl. It read a response code as opaque text, so a missing script and a quota were both "the server said no". It never sent RENAMESCRIPT, CHECKSCRIPT's sibling that RFC 5804 defines in the same breath, nor NOOP nor UNAUTHENTICATE.

## What

Extract the protocol into io-managesieve, a new I/O-free library shaped like io-imap and io-smtp: one coroutine per RFC 5804 command, a composite session-opening coroutine, and an optional std client behind the `client` feature. Himalaya then keeps a `SieveClient` wrapping `ManagesieveClientStd`, one file per subcommand, and its own serializable output types, which is what every other protocol module already looks like.

Take the coverage the extraction makes free. Every SASL mechanism io-sasl computes reaches ManageSieve, since RFC 5804 frames them all identically and one coroutine can therefore serve all of them, server-first ones included. Response codes are parsed, so a caller can tell NONEXISTENT from QUOTA. `sieve rename` joins the command list.

Keep the two guards the in-repo client grew, and move them where they cannot be forgotten. Refusing to send a password over a cleartext connection is what RFC 5804 section 5 asks implementations to carry, so it lives in the session coroutine with `sieve.allow-cleartext-auth` to opt out. Refusing bytes that arrive past a greeting or past a STARTTLS reply is the injection defence io-imap and io-smtp already have, and the coroutine fails rather than handing them back.

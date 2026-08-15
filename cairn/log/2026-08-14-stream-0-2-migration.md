---
cairn: log
change: stream-0-2-migration
landed: 2026-08-14
---

# pimalaya-stream 0.2 migration

himalaya and four io-* crates moved to pimalaya-stream 0.2, unblocking the io-imap and io-smtp updates that had been stuck behind it.

The blockage was a diamond, not a compile error. io-imap master required stream `^0.2` while io-gmail, io-jmap, io-msgraph and io-pim-discovery still required `^0.1`. A `[patch.crates-io]` entry replaces a crate globally with one version, so patching stream to git did not unify them: cargo simply resolved both 0.1.2 and 0.2.0 into the graph. Two copies of a crate are two distinct types, and the wizard failed where io-pim-discovery's re-exported `Tls` met himalaya's.

Moving the four turned out to be free. Stream 0.2's only breaking change is that the `sasl` module left for the io-sasl crate, and none of the four used it; the `std` and `tls` public APIs are byte-identical to 0.1.2. So each is a one-line dependency bump, a patch table for stream and io-http, and a changelog entry. No code changed in any of them. Their crate versions are deliberately left alone so himalaya's existing requirements still match through its patch table.

himalaya absorbed three upstream API changes at once. The SASL types moved to io-sasl and the credential structs gained a `Creds` suffix, `SaslPlain` now being the coroutine and `SaslPlainCreds` the credentials; the three SCRAM profiles share one `SaslScramCreds` carrying the client nonce and the channel binding, an I/O-free coroutine having no way to draw randomness, so the config passes an empty nonce for the client to fill. `ImapClientStdConnectOptions` became `ImapSessionOpenOptions` from the session module. The io-imap and io-smtp command methods moved onto `ImapClient` and `SmtpClient` traits, brought into scope at the call sites as anonymous imports since himalaya has its own wrapper also called `ImapClient`. Smaller signature changes came with them: `SmtpClientStd::connect` takes starttls in an options struct and returns the client alongside the EHLO capabilities, `status` takes a `Cow`, `imap raw` takes bytes and `smtp raw` a `Cow`.

The wizard still offers the same six SASL mechanisms. io-sasl knows sixteen, so both matches over `SaslMechanism` gained a catch-all naming the mechanism, rather than accepting one the config cannot express.

Deliberately not done: nothing is released and no crate version is bumped. io-oauth also still requires stream `^0.1`, but nothing in himalaya's graph reaches it.

Verified: one pimalaya-stream in the lock, down from two; build, fmt and clippy clean; 88 tests pass. Not verified against live servers.

Left open: himalaya's patch table points the four crates at local paths, since their bumps are not pushed. They must become git entries before this is committed.

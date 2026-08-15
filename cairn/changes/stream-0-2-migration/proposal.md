---
cairn: change
id: stream-0-2-migration
status: landed
created: 2026-08-14
---

# pimalaya-stream 0.2 migration

## Why

io-imap master moved to `pimalaya-stream ^0.2`, which is unpublished, so himalaya could not take any io-imap update. Patching stream to git did not resolve it: a patch replaces a crate globally with one version, and io-gmail, io-jmap, io-msgraph and io-pim-discovery all required `^0.1`, so cargo resolved both 0.1.2 and 0.2.0 into the graph. Two copies of the same crate means two incompatible `Tls` types, and the wizard stopped compiling where one met the other.

Stream 0.2 is a small release with one breaking change: the `sasl` module moved out to the io-sasl crate. The `std` and `tls` module APIs are byte-identical to 0.1.2, which is what made the four blocked crates cheap to move.

io-imap and io-smtp had each also grown a client trait layer in the same window, so taking their new commits meant absorbing that too.

## What

The four blocked crates take stream 0.2. None of them used `pimalaya_stream::sasl`, so each is a one-line dependency bump with no code change.

himalaya then absorbs three upstream API changes: `pimalaya_stream::sasl` becomes io-sasl (with the credential structs renamed `Sasl*Creds` and SCRAM's nonce and channel binding now carried on the credentials), `ImapClientStdConnectOptions` becomes `ImapSessionOpenOptions`, and the io-imap and io-smtp command methods move onto `ImapClient` and `SmtpClient` traits that call sites bring into scope. `SmtpClientStd::connect` takes its starttls flag in an options struct and returns the client with the EHLO capabilities.

## Scope / non-goals

No crate versions are bumped and nothing is released. The four crates keep the versions they published under so himalaya's existing `version = "0.2"` style requirements still match through the patch table.

The wizard keeps offering the same six SASL mechanisms. io-sasl knows sixteen, so the two matches over `SaslMechanism` gain a catch-all that names the mechanism rather than silently accepting one the config cannot express.

io-oauth also still requires stream `^0.1`, but nothing in himalaya's graph pulls it, so it is left alone.

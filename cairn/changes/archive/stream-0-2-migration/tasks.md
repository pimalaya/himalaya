---
cairn: tasks
change: stream-0-2-migration
---

# Tasks

- [x] Bump pimalaya-stream to 0.2 in io-gmail, io-jmap, io-msgraph and io-pim-discovery, with a patch table for stream and io-http, and a changelog entry in each.
- [x] Confirm all four still build with no code change, the stream `std` and `tls` APIs being unchanged.
- [x] himalaya: add io-sasl, move the `Sasl` types off pimalaya-stream and rename them to `Sasl*Creds`.
- [x] himalaya: supply the SCRAM nonce (empty, drawn by the client) and channel binding the credentials now carry.
- [x] himalaya: `ImapClientStdConnectOptions` to `ImapSessionOpenOptions`; bring `ImapClient` and `SmtpClient` into scope at the call sites.
- [x] himalaya: `SmtpClientStd::connect` options struct and tuple return; `status` takes a `Cow`; `imap raw` takes bytes; `smtp raw` takes a `Cow`.
- [x] himalaya: catch-all arms for the ten SASL mechanisms the config cannot express.
- [x] Build/test/fmt/clippy.
- [x] Log; land.

---
cairn: log
change: live-provider-tests
landed: 2026-11-11
---

# Live provider tests

tests/google.rs, tests/microsoft.rs and tests/fastmail.rs run the built binary against the real providers, every test ignored unless `--ignored` is given. tests/common.rs holds what they share: `write_config`, `himalaya` (the binary with `HIMALAYA_TEST_TOKEN` in its environment, its home a TempDir, stdin closed), `failure`, `json`, `json_failure`, `json_check_failure`, `assert_mailboxes`, `assert_envelopes`, `sample_section`, `env_var`, `require_env` (every unset name in one failure) and `env_content` (a `_FILE` variable or the content itself). Each account reads its token through `["printenv", "HIMALAYA_TEST_TOKEN"]`. The existing suites are untouched.

- `anonymous` (a made-up token, no account touched): Google `gmail_account_check_refused`, `gmail_envelope_list_refused` on the sample's Gmail section, `imap_account_check_refused`, `imap_envelope_list_refused` on a Gmail IMAP XOAUTH2 account written in the suite, the sample having no Gmail IMAP section; Microsoft `graph_account_check_refused`, `graph_envelope_list_refused` on the Microsoft Graph section, `imap_smtp_account_check_refused` (both backends of the Microsoft IMAP and SMTP section fail), `imap_envelope_list_refused`; Fastmail `jmap_account_check_refused`, `jmap_envelope_list_refused`. `account check` exits 2 with every backend failed, `envelope list` exits 1 with the failure object, and the message names the refusal in Himalaya's words: `HTTP 401` (Gmail API, Graph, JMAP), `XOAUTH2 failed` (IMAP, SMTP). The failure `code` is `null` in every case, no stable code naming an authentication refusal.
- `credentialed` (read-only): `account check`, `mailbox list` (an inbox role) and `envelope list -s 5` (one to five envelopes of the printed shape) per backend, one token minted per run with io-oauth 0.5.2, a new dev-dependency (`client`, `jwt-bearer`, `rustls-ring`). Microsoft `graph_*` on a client credentials token for `https://graph.microsoft.com/.default` with the sample's `msgraph.user-id` set to microsoft@pimalaya.onmicrosoft.com, and `imap_*` on one for `https://outlook.office365.com/.default`: the tenant accepts app-only XOAUTH2 on Exchange IMAP, so IMAP is in. Google `gmail_*` and `imap_*` on a service account token for `https://mail.google.com/` acting as google@pimalaya.org (RFC 7523 section 2.1).

.github/workflows/tests.yml keeps the shared tests job and gains a `live` matrix job (google, microsoft, fastmail, `fail-fast: false`, `RUST_LOG: debug`): the anonymous step on every push, the credentialed steps in pimalaya/himalaya only and when `HAS_GOOGLE` or `HAS_MICROSOFT` holds, mapping `GOOGLE_SERVICE_ACCOUNT_KEY`, `MICROSOFT_TENANT_ID`, `MICROSOFT_CLIENT_ID` and `MICROSOFT_CLIENT_SECRET` to their `HIMALAYA_*` variables. actionlint passes.

Run on 2026-11-11 from this machine: anonymous 10 passed (Google 4, Microsoft 4, Fastmail 2), credentialed 12 passed (Google 6, Microsoft 6), each envelope page holding 5 envelopes; Microsoft with the tenant name `pimalaya.onmicrosoft.com` in place of its id, Google with the key's `_FILE` path. One run of all 22 at `RUST_LOG=debug` saved each binary's stderr: debug lines in every one, and no hit for the minted tokens (counted in the test process), the client secret, the made-up token, a `ya29.` prefix or a JWT. A credentialed test without its variables fails naming each one unset.

io-oauth brings io-http 0.6.0 from crates.io beside the patched io-http 0.5.0 the binary uses, rsa 0.10 and its key crates, and moves ctutils from 0.4.7 to 0.4.8, all in the test build only. `cargo deny check` passes, its duplicate warnings unchanged.

cairn/spec/testing/imap-smtp-posteo.md is removed with its row in cairn/spec/testing/README.md, which now points at the live suites: it was run against the owner's personal account, which no test may touch.

Capabilities moved: packaging (cairn/spec/packaging.md: "Live provider suites" added); testing (the Posteo report removed).

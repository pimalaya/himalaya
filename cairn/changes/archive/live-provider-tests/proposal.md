---
cairn: change
id: live-provider-tests
status: landed
created: 2026-11-11
---

# Live provider tests (Google, Microsoft, Fastmail)

## Why

Every integration suite under tests/ runs the binary against a local store or a port nothing listens on. No test runs it against a real provider, so a sample section that parses (the-sample-is-tested-and-shipped) can still fail at Google, Microsoft or Fastmail: a moved endpoint, a refused authentication the error path mangles, an exit code off the output contract, a JSON shape a provider's answer breaks. The provider checks so far are the manual reports under cairn/spec/testing/, run once by hand. Ortie landed the same kind of suites on 2026-11-11 (ortie/cairn/log/2026-11-11-live-provider-tests.md); Himalaya follows its shape.

## What

tests/google.rs, tests/microsoft.rs and tests/fastmail.rs, every test `#[ignore = "requires ..."]` and run with `--ignored`, sharing a new tests/common.rs: `write_config` (one default account), `himalaya` (the built binary, a token in its environment, its home a TempDir), `json`, `json_failure` and `json_check_failure` (exit code then JSON), `assert_mailboxes` and `assert_envelopes`, `sample_section` (a provider section of config.sample.toml, uncommented, keys overridable), `env_var`, `require_env` and `env_content` (an empty variable counts as unset, `_FILE` names a file). Every account reads its token through `["printenv", "HIMALAYA_TEST_TOKEN"]`, so a token reaches the binary through its environment only, never argv nor the configuration file.

### anonymous (network only, a made-up token, no account touched)

For each backend a provider's sample offers, `account check` exits 2 with every backend of its report failed, and `envelope list` exits 1 with the failure object, both naming the refusal in Himalaya's words (`HTTP 401` for the Gmail API, Graph and JMAP, `XOAUTH2 failed` for IMAP and SMTP), never the provider's prose.

- Google: the Gmail section (Gmail API), and a Gmail IMAP account on XOAUTH2, which the sample has no section for.
- Microsoft: the Microsoft Graph section, and the Microsoft IMAP and SMTP section, whose check covers both.
- Fastmail: the Fastmail section (JMAP).

### credentialed (read-only, upstream CI only)

`account check`, `mailbox list --json` (an inbox role among the mailboxes) and `envelope list --json -s 5` (one to five envelopes of the printed shape) per backend, each test sharing one token minted per run with io-oauth (a new dev-dependency, `client`, `jwt-bearer`, `rustls-ring`):

- Microsoft: client credentials of the test registration (`HIMALAYA_MICROSOFT_TENANT_ID`, `HIMALAYA_MICROSOFT_CLIENT_ID`, `HIMALAYA_MICROSOFT_CLIENT_SECRET`), on the mailbox `HIMALAYA_MICROSOFT_USER` (default microsoft@pimalaya.onmicrosoft.com): Graph on a `https://graph.microsoft.com/.default` token through the sample's `msgraph.user-id`, and Exchange IMAP on a `https://outlook.office365.com/.default` token, which the tenant accepts.
- Google: the RFC 7523 section 2.1 grant of the service account (`HIMALAYA_GOOGLE_SERVICE_ACCOUNT_KEY` or its `_FILE`) acting as `HIMALAYA_GOOGLE_SERVICE_ACCOUNT_SUBJECT` (default google@pimalaya.org), scope `https://mail.google.com/`: the Gmail API and Gmail IMAP.
- Fastmail: none, only a browser consent issues its tokens.

A credentialed test missing its variables fails naming them.

### CI

.github/workflows/tests.yml keeps the shared tests job and gains a `live` matrix job (google, microsoft, fastmail, `fail-fast: false`) at `RUST_LOG=debug`: the anonymous step on every push, the credentialed steps in pimalaya/himalaya only and once their secrets exist (`HAS_GOOGLE`, `HAS_MICROSOFT`), mapping the org secrets `GOOGLE_SERVICE_ACCOUNT_KEY`, `MICROSOFT_TENANT_ID`, `MICROSOFT_CLIENT_ID` and `MICROSOFT_CLIENT_SECRET` to the `HIMALAYA_*` variables.

### The Posteo report leaves

cairn/spec/testing/imap-smtp-posteo.md was run against the owner's personal account, which no test may touch; it leaves, with its row in cairn/spec/testing/README.md.

## Out of scope

- Writes of any kind (send, move, flag, delete, mailbox create), on any account.
- SMTP with a real token, and every provider not listed: their manual reports stay.
- Moving the existing suites onto tests/common.rs.

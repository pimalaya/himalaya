---
cairn: delta
change: the-tui-runs-on-himalaya
---

# Delta

## ADDED Requirements

### Requirement: A binary, and a library for himalaya-tui
(packaging) Himalaya SHALL build as a binary, and as a library whose only consumer is himalaya-tui: src/lib.rs declaring the modules, src/main.rs holding `main` alone over it. The library SHALL carry no semver promise of its own, the contract being the CLI's grammar and its versioned `--json` output, and a consumer SHALL pin it by git revision. A user needing the protocol or storage logic still reaches for the io-* library that owns it (io-imap, io-jmap, io-gmail, io-msgraph, io-smtp, io-managesieve, io-maildir, io-mbox, io-pimdir).

### Requirement: An envelope listing says how many messages the mailbox holds
(commands) `envelope list` SHALL print `total` beside `envelopes`, `nextPage` and `errors`: how many messages the mailbox holds, when the backend answers it within the listing's own exchanges (the `EXISTS` of IMAP's `SELECT`, JMAP's `calculateTotal`, the entries Maildir, mbox and pimdir list), and `null` otherwise, Gmail among them. No backend SHALL send a request for the total alone. `envelope search`, sharing that output, SHALL print `total` as `null`, a search counting its matches rather than the mailbox. The plain text output does not change.

#### Scenario: An IMAP mailbox of 120 messages
- GIVEN an IMAP mailbox holding 120 messages
- WHEN `himalaya --json envelope list -s 25` runs
- THEN it prints 25 envelopes, `nextPage` `"2"` and `total` 120

#### Scenario: Gmail
- GIVEN a Gmail account
- WHEN `himalaya --json envelope list` runs
- THEN `total` is `null`

### Requirement: A session can be kept alive
(repl) `Session::keep_alive` SHALL send `NOOP` on the unified client's IMAP connection when the session holds one, and SHALL do nothing when the slot is empty or the backend keeps no connection. A keepalive that fails SHALL empty the slot, as a failed command does, so the next command builds the client again. No command of the grammar SHALL call it. `Session::is_connected` SHALL say whether the unified client is built, so a caller knows the next command connects first.

#### Scenario: An idle IMAP connection
- GIVEN a session whose unified client holds an IMAP connection
- WHEN `Session::keep_alive` runs
- THEN one `NOOP` is sent on that connection and the client is kept

#### Scenario: A dead connection
- GIVEN a session whose IMAP connection the server closed
- WHEN `Session::keep_alive` runs
- THEN it fails, the slot is emptied, and the next command connects again

## MODIFIED Requirements

## REMOVED Requirements

### Requirement: Binary only
(packaging) Replaced by "A binary, and a library for himalaya-tui": the owner decided on 2026-11-14 that himalaya-tui links himalaya.

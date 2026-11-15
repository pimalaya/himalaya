---
cairn: delta
change: an-account-says-what-it-serves
---

Folded into cairn/spec/commands.md on 2026-11-03.

## ADDED Requirements

### Requirement: An account lists what it serves
`account capability` SHALL print one row per shared capability, `{name, backend, level, detail}` under `{account, capabilities}`: the backend the resolver picks for the account and `--backend`, `null` when none is configured, and that backend's level and detail. It SHALL read the configuration and the capability tables and connect nowhere. On a pimdir account, a write SHALL be read from what the store's sources declare: an item write through the store's gate in each of the account's mail collections (`none` refused in all, `partial` refused in some or passing in part), an intent served when a source performs it, the static row kept when no source declares anything.

#### Scenario: A Maildir synced from JMAP
- GIVEN an account configuring a Maildir and a JMAP block
- WHEN `account capability` runs
- THEN `mailbox.list` names `maildir`, `vacation` names `jmap` at `server`, and nothing connects

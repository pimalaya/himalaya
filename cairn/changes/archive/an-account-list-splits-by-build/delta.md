---
cairn: delta
change: an-account-list-splits-by-build
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: account check covers every declared backend
`account check` SHALL check every backend the account declares and `--backend` allows, and print the report whole, pimalaya-cli's `account::AccountCheckOutput`, one row `{backend, ok, code, error}` per backend, `code` and `error` `null` when the backend answered, a failure coded as a one-shot run codes it. When any fails, it SHALL exit 3 after the report, so a script reads the outcome from the exit code and each backend's code from its row. A pimdir block SHALL be checked by opening the store as a reader, listing its mailboxes, then opening a producer and probing it (`PimdirProducer::probe`), which takes and releases the staging lock without waiting and the database's write lock, writing nothing into the store, so an account declaring only a pimdir block is checked rather than refused as declaring no backend, and a store a write cannot reach fails the check rather than the first write: a collector holding the staging lock fails it at once with `store-busy`, and the check leaves nothing in objects/. A Maildir block SHALL be checked by listing its store, a missing root failing the check.

### Requirement: An account list names every block
`account list` SHALL print pimalaya-cli's `account::AccountListOutput`, one row per account, `{name, default, backends, notBuilt}`, sorted by name: `backends` naming each block the account declares that the build serves, `notBuilt` each block it declares that the build leaves out, both among `imap`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox`, `pimdir`, `smtp` and `sieve`, in that order. Its text SHALL be pimalaya-cli's: the name, the backends joined with `,`, each one left out followed by ` (not built)`, and `default` on the default account. It SHALL load the configuration without taking an account.

#### Scenario: A pimdir store
- GIVEN an account declaring a `pimdir` block and an `smtp` block
- WHEN `account list` runs
- THEN its BACKENDS cell reads `pimdir,smtp`

#### Scenario: A backend the build leaves out
- GIVEN a build without `mbox` and an account declaring a `maildir` block and an `mbox` block
- WHEN `account list` runs
- THEN its row prints `"backends":["maildir"],"notBuilt":["mbox"]` under `--json`, and its BACKENDS cell reads `maildir,mbox (not built)`

## REMOVED Requirements

None.

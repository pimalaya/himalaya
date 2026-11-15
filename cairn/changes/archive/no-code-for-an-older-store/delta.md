---
cairn: delta
change: no-code-for-an-older-store
---

## MODIFIED Requirements

### Requirement: A JSON error carries a stable code
A failure a caller is expected to act on SHALL carry a stable code, printed under `--json` as a `code` string beside `error`, `sources` and `backtrace`. The code SHALL be found anywhere in the error chain, so added context does not hide it. A failure with no code SHALL print no `code` field. The plain output SHALL stay unchanged. Codes are kebab-case and never renamed; `error` stays free wording.

### Requirement: A pimdir store failure carries its code
A failure io-pimdir reports SHALL keep io-pimdir's message as a cause and carry, under `--json`, the code `PimdirError::code()` gives it, spelled the same, when it is one a caller acts on: `store-newer`, `store-uncreated`, `store-foreign`, `store-corrupt`, `store-not-current`, `store-stale`, `store-hash-algo`, `store-busy`, `store-owned`, `store-staging`, `store-in-flight`, `item-duplicate`, `unsupported`, `no-performer` and `ambiguous-performer`. An SQL, I/O or JSON failure and a malformed action carry none. Himalaya SHALL NOT derive a code from a message, and SHALL NOT advise deleting the store: a store newer than Himalaya's schema says to upgrade Himalaya, an older, stale or reshaped one to run or upgrade the store's sync engine.

#### Scenario: A store a newer sync engine wrote
- GIVEN a pimdir store stamped with a schema version above Himalaya's
- WHEN any pimdir command opens it
- THEN the command fails with the code `store-newer`, saying to upgrade this program

#### Scenario: A draft the mailbox holds already
- GIVEN a pimdir mailbox holding a message whose `Message-ID` is `d1@x.org`
- WHEN `himalaya message add -m imap/Drafts` stages a message with that `Message-ID`
- THEN the command fails with the code `item-duplicate`, and nothing is queued


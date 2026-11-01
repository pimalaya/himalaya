---
cairn: delta
change: pimdir-adopts-the-io-pimdir-release
---

## ADDED Requirements

### Requirement: A pimdir store failure carries its code (backends)
A failure io-pimdir reports SHALL keep io-pimdir's message as a cause and carry, under `--json`, the code `PimdirError::code()` gives it, spelled the same, when it is one a caller acts on: `store-newer`, `store-older`, `store-uncreated`, `store-corrupt`, `store-not-current`, `store-stale`, `store-hash-algo`, `store-busy`, `store-owned`, `store-staging`, `store-in-flight`, `item-duplicate`, `unsupported`, `no-performer` and `ambiguous-performer`. An SQL, I/O or JSON failure and a malformed action carry none. Himalaya SHALL NOT derive a code from a message, and SHALL NOT advise deleting the store: a store newer than Himalaya's schema says to upgrade Himalaya, an older, stale or reshaped one to run or upgrade the store's sync engine.

#### Scenario: A store a newer sync engine wrote
- GIVEN a pimdir store stamped with a schema version above Himalaya's
- WHEN any pimdir command opens it
- THEN the command fails with the code `store-newer`, saying to upgrade this program

### Requirement: account check covers every declared backend (commands)
`account check` SHALL check every backend the account declares and `--backend` allows, reporting each OK or FAIL with why. A pimdir block SHALL be checked by opening the store as a reader, listing its mailboxes, then opening and dropping a producer, so an account declaring only a pimdir block is checked rather than refused as declaring no backend.

## MODIFIED Requirements

### Requirement: A JSON error carries a stable code (commands)
The code table gains the pimdir store codes of "A pimdir store failure carries its code".

### Requirement: pimdir is a reader and a producer, never the owner (backends, the added message paragraph)
An added message whose link id the mailbox already holds live, or has an `add` queued under, SHALL be refused with the code `item-duplicate` through io-pimdir's `enqueue_add`, and nothing SHALL be queued: the owner would park such a row rather than deduplicate or mint a key of its own. An `add` still parks when the source lists the same `Message-ID` after it was staged.

### Requirement: A pimdir body is written under the staging lock (backends)
A pimdir write carrying a body (`add_message`, `send_message`) SHALL stage it through its producer (`stage_body`), which hashes it under the store's algorithm and writes it inside the producer's staging window (pimdir STORAGE §8), and SHALL enqueue the action that pins it before the producer drops.

### Requirement: The pimdir queue view shows parked rows (backends, scenario)
The duplicate-draft scenario parks an `add` the source listed a twin of before the owner applied it, a duplicate known at staging being refused.

### Requirement: pimdir sends by queueing a submit intent (backends)
The payload SHALL be built by io-pimdir's `PimdirSubmit` (STORAGE Annex B.2): `v: 1` JSON carrying `source` when a performer is named, `object` (the body hash), `from`, `rcpts`, `subject`, and `copy` when the intent carries the copy, the keys and values a performer reads back with `PimdirSubmit::from_payload`.

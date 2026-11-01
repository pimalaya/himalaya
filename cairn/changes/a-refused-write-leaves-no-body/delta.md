---
cairn: delta
change: a-refused-write-leaves-no-body
---

## MODIFIED Requirements

### Requirement: A pimdir body is written under the staging lock
A pimdir write carrying a body (`add_message`, `send_message`) SHALL hand the body to its producer with the action in one step (`enqueue_with_body_checked`): the capability gate and, for an add, the duplicate check run on the bytes in hand, then the body is written inside the producer's staging window (pimdir STORAGE §8) and the row pinning it appended, in one transaction. A write refused by the gate or the duplicate check SHALL leave no body in the blob tree. No body SHALL reach the blob tree while a collector holds the lock: the write waits for it.

#### Scenario: A collector running beside a staged add
- GIVEN a pimdir store whose collector holds `objects.lock` exclusively
- WHEN a message is added to a mailbox
- THEN no body is written until the collector releases the lock, and then the body lands and its `add` is queued

#### Scenario: A refused add
- GIVEN a pimdir mailbox holding a message whose `Message-ID` is `d1@x.org`
- WHEN another body with that `Message-ID` is added to it
- THEN the add fails with `item-duplicate` and its body is not in the blob tree

### Requirement: A pimdir store failure carries its code
A failure io-pimdir reports SHALL keep io-pimdir's message as a cause and carry, under `--json`, the code `PimdirError::code()` gives it, spelled the same, when it is one a caller acts on: `store-newer`, `store-older`, `store-uncreated`, `store-foreign`, `store-corrupt`, `store-not-current`, `store-stale`, `store-hash-algo`, `store-busy`, `store-owned`, `store-staging`, `store-in-flight`, `item-duplicate`, `unsupported`, `no-performer` and `ambiguous-performer`. An SQL, I/O or JSON failure and a malformed action carry none. Himalaya SHALL NOT derive a code from a message, and SHALL NOT advise deleting the store: a store newer than Himalaya's schema says to upgrade Himalaya, an older, stale or reshaped one to run or upgrade the store's sync engine.

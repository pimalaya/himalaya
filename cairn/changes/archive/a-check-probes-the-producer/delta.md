---
cairn: change
change: a-check-probes-the-producer
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: account check covers every declared backend
`account check` SHALL check every backend the account declares and `--backend` allows, reporting each OK or FAIL with why. A pimdir block SHALL be checked by opening the store as a reader, listing its mailboxes, then opening a producer and probing it (`PimdirProducer::probe`), which takes and releases the staging lock without waiting and the database's write lock, writing nothing into the store, so an account declaring only a pimdir block is checked rather than refused as declaring no backend, and a store a write cannot reach fails the check rather than the first write: a collector holding the staging lock fails it at once with `store-busy`, and the check leaves nothing in objects/.

## REMOVED Requirements

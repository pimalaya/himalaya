---
cairn: delta
change: account-check-fails-on-a-failed-backend
---

## MODIFIED Requirements

### Requirement: account check covers every declared backend
`account check` SHALL check every backend the account declares and `--backend` allows, printing one verdict per backend when all of them pass. When any fails, it SHALL exit non-zero with one error naming each failed backend and why, so a script reads the outcome from the exit code alone, as calendula and cardamum do. A pimdir block SHALL be checked by opening the store as a reader, listing its mailboxes, then opening a producer and probing it (`PimdirProducer::probe`), which takes and releases the staging lock without waiting and the database's write lock, writing nothing into the store, so an account declaring only a pimdir block is checked rather than refused as declaring no backend, and a store a write cannot reach fails the check rather than the first write: a collector holding the staging lock fails it at once with `store-busy`, and the check leaves nothing in objects/.


---
cairn: tasks
change: pimdir-producer-reader
---

- [x] io-pimdir 0.3 and io-replica 0.4, patched to git
- [x] Reads open the store read-only, taking no lock
- [x] Writes enqueue `PimdirAction`s through `PimdirProducer`
- [x] An added body is written and pinned before its action is enqueued
- [x] Mailboxes display and resolve by their server name
- [x] An unknown or ambiguous mailbox errors naming the candidates
- [x] `pimdir.account` replaces `pimdir.source`; `pimdir.namespace` overrides the derivation
- [x] `conventions` replaces the local link/meta derivation, `mid:` translated at the seam
- [x] Test: an unknown flag set renders as no flags and stages as a known one
- [x] Test: an added message links the way the store spells it, `alt:` passed through
- [x] Verified against a real 8824-item Neverest store

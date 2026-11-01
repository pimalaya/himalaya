---
cairn: log
change: a-queued-write-says-queued
landed: 2026-10-30
---

# A queued write says it is queued

From the pimdir stable-release review of 2026-10-30: on a pimdir account the shared commands confirmed a send, a delete or a move as done when only a queue row existed. `EmailClient::queues_writes` says whether the storage backend queues its writes for a sync engine (pimdir), and the plain confirmations of `message send`, `message add`, the composers' save and send, `message delete`, `message move`, `message copy` and `flag add|set|remove` say "queued" there. The JSON shapes are unchanged: the flag lives on the outputs as a `serde(skip)` field, and only the prose of a `message` confirmation changes. This reverses the 2026-08-27 choice of one wording for every backend, on the plan's Phase A decision; no pimdir detail reaches a shared output.

Tests: the queued and done renderings of `FlagAddOutput`, `MessageDeleteOutput` and `MessageAddOutput`, each checking the JSON is the same either way.

Capabilities moved: **commands** (A queued write says it is queued).

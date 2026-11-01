---
cairn: tasks
change: a-check-probes-the-producer
---

# Tasks

- [x] Test: a held staging lock fails the pimdir check with `store-busy` and leaves objects/ empty; the check passes once the lock goes (failed on the open-and-drop check: it passed under the lock).
- [x] `connect_pimdir` probes the producer.
- [x] CHANGELOG `account check` bullet.
- [x] Fold the delta into [commands](../../spec/commands.md); log.

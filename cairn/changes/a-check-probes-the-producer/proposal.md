---
cairn: change
id: a-check-probes-the-producer
status: landed
created: 2026-10-30
---

# A check probes the producer

## Why

`account check` on pimdir opens a producer and drops it, to prove a write can reach the store. Since io-pimdir's N8 change (a-producer-proves-it-can-write, 2026-10-30, io-pimdir/cairn/log/2026-10-30-a-producer-proves-it-can-write.md) a producer takes the staging lock at its first staged body, not at its open, so that proves nothing: a store whose staging lock a collector holds passes the check and the first write waits on it. io-pimdir now offers `PimdirProducer::probe`, which takes and releases the staging lock without waiting and the database's write lock, writing nothing. cardamum, calendula and unfog align the same way.

## What

- `connect_pimdir` (src/account/check.rs) probes the producer it opens; a failure carries io-pimdir's code through `pimdir_error`, `store-busy` when a collector holds the staging lock.

---
cairn: change
id: a-staged-body-is-written-under-the-lock
status: landed
created: 2026-10-30
---

# A staged body is written under the staging lock

## Why

The pimdir stable-release review of 2026-10-30 (report-himalaya.md, release blockers 2 and 3) found two problems in the pimdir write path.

- `add_message` and `send_message` write the body through the reader's `PimdirBlobs` (src/pimdir/backend.rs:500, :558) and only then open the producer (:507, :565), which takes `objects.lock` shared. strace of `pimdir message add` shows `rename` into objects/ before `flock(LOCK_SH)`. pimdir STORAGE §8 requires the producer to hold the staging lock across the blob write and the enqueue that pins it: a collector running in that window deletes the body of an outgoing message, and the enqueue then pins an object row with no file.
- Himalaya pins io-pimdir 3b4bed8 while every other pimdir app moves to one revision, 4c3353e. That revision adds `PimdirAction::Add.supersedes`, so himalaya does not compile against it.

cardamum's `stage` (src/pimdir/backend.rs:416-447) is the model: open the producer first, then write the body.

## What

- Pin io-pimdir to 4c3353e7eb91c834b222107fac333970f2761b11 in `[patch.crates-io]` (`cargo update -p io-pimdir` only) and add `supersedes: None` where himalaya builds an `Add`: it never replaces an item.
- `add_message` and `send_message` open the producer first and write the body through `producer.blobs()`, under the staging lock, then enqueue.
- A regression test holding `objects.lock` exclusively, as a collector does, proves no body reaches the blob tree while it is held, and that the add completes once it is released.

io-pimdir's coming `PimdirProducer::stage_body` (hashing and writing under the lock in one call) replaces the hand-written pair later: pimdir-adopts-the-io-pimdir-release.

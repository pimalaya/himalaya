---
cairn: log
change: a-staged-body-is-written-under-the-lock
landed: 2026-10-30
---

# A staged body is written under the staging lock

From the pimdir stable-release review of 2026-10-30. `add_message` and `send_message` wrote the body through the reader's blob directory and only then opened the producer, whose open takes `objects.lock` shared, so a collector running in between could delete the body of a queued message and the enqueue then pinned an object with no file (pimdir STORAGE §8). Both now open the producer first, write through `producer.blobs()` with the producer's hash, and enqueue before it drops; `send_message` also names its performer before writing, so a send refused for want of one writes no body.

io-pimdir is pinned to 4c3353e7eb91c834b222107fac333970f2761b11, the revision every pimdir app moves to, with `cargo update -p io-pimdir` alone (cargo also moved a few crates from windows-sys 0.52 to 0.61 in the lockfile on its own). The only break was the new `PimdirAction::Add.supersedes`, `None` where Himalaya builds an add: it never replaces an item.

Tests: `an_added_body_waits_for_the_collector` and `a_sent_body_waits_for_the_collector` hold `objects.lock` exclusively, as the collector does, and check no body reaches the blob tree until it is released, then that the row lands. Both failed on the old order with "a body was written while the collector held the lock".

Capabilities moved: **backends** (A pimdir body is written under the staging lock).

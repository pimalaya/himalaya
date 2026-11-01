---
cairn: log
change: a-check-probes-the-producer
landed: 2026-10-30
---

# A check probes the producer

`connect_pimdir` (src/account/check.rs) calls `PimdirProducer::probe` (io-pimdir a-producer-proves-it-can-write, 2026-10-30) on the producer it used to open and drop, its failure mapped through `pimdir_error`. Since io-pimdir N8 an open producer holds no staging lock until its first body, so the open-and-drop check passed while a collector held the lock; the probe takes and releases it without waiting, and the database's write lock, writing nothing, and a held lock fails the check at once with `store-busy`.

Test: `a_held_staging_lock_fails_the_check_as_busy` holds objects.lock exclusively, as a collector does, and checks the check fails with `store-busy`, passes once the lock goes, and leaves objects/ empty. On the old check: `panicked at src/account/check.rs:676:43: called Result::unwrap_err() on an Ok value: ()`.

Capabilities moved: **commands** (account check covers every declared backend, modified).

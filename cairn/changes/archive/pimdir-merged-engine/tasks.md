---
cairn: tasks
change: pimdir-merged-engine
---

- [x] Drop `dep:io-replica` from the `pimdir` feature, bump io-pimdir to 0.4 behind a path patch, remove the io-replica patch
- [x] Confirm `mail-parser` is still pulled by the shared code a pimdir-only build compiles
- [x] Port the client onto `io_pimdir::client::{PimdirStore, PimdirError, reader, producer, blobs}`
- [x] Build envelopes from `PimdirSummary::Mail`, addresses and display names included; delete `MetaView`
- [x] List through `list_summaries` in the store's newest-first order; drop the in-memory date sort
- [x] Derive an added message's link id through `summary::mail::derive`; enqueue without a timestamp
- [x] Derive a queued creation's row from the body the action pins
- [x] Restrict a mailbox to a collection declared `message/rfc822`
- [x] Build, test, clippy and fmt under the default and the pimdir-only feature sets
- [x] Fold the spec, log the change, update the CHANGELOG

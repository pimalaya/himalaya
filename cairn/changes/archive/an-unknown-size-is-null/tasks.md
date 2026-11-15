---
cairn: tasks
change: an-unknown-size-is-null
---

# Tasks

- [x] src/email/envelope.rs: `Envelope::size` an `Option<u64>`; Graph `None`, JMAP and pimdir passing their own `Option`, IMAP `RFC822.SIZE` when fetched, Maildir, mbox and Gmail as before; src/pimdir/backend.rs `PimdirQueuedEnvelope::size` too.
- [x] Test: Graph `an_envelope_size_graph_does_not_give_is_null`, failing on the old code (`0`).
- [x] Fold into cairn/spec/backends.md; CHANGELOG; MIGRATION.md; the log entry.

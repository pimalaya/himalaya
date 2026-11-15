---
cairn: log
change: an-unknown-size-is-null
landed: 2026-11-15
---

# An unknown size is null

Task H22 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (msgraph-microsoft.md, H-4): Graph envelopes printed `"size":0`.

## What landed

- src/email/envelope.rs: `Envelope::size` is an `Option<u64>`, its schema `integer` or `null`. src/msgraph/backend.rs `envelope_from` gives `None`; src/jmap/backend.rs and src/pimdir/backend.rs pass the size they hold, `None` where they hold none, where they wrote 0; src/imap/backend.rs gives `RFC822.SIZE` when the fetch carries it; Maildir, mbox and Gmail (`sizeEstimate`) as before. `PimdirQueuedEnvelope::size` follows.
- Test: `an_envelope_size_graph_does_not_give_is_null`.

## Capabilities moved

- [backends](../spec/backends.md): *Graph envelopes* names a `null` size.

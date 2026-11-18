---
cairn: log
change: imap-thread-fetches-in-chunks
landed: 2026-11-18
---

# imap thread fetches in chunks

Finding HIM-23 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/imap/envelope/thread.rs: `fetch_envelopes` in chunks of 255.
- Test `a_long_thread_listing_fetches_in_chunks`, failing on 002bf012.
- CHANGELOG (Fixed).

## Capabilities moved

- [backends](../spec/backends.md): *imap thread fetches its envelopes in chunks*, new.

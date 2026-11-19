---
cairn: log
change: the-id-lists-scale
landed: 2026-11-19
---

# The id lists scale

Finding HIM-16 of the family's cli-family-analysis-2026-11-15 plan, its remainder; with the-mbox-cache-saves-in-one-pass (2026-11-18) the finding is closed in Himalaya.

## What landed

- src/shared/flag/arg.rs: the ids deduplicated through a `HashSet`.
- src/email/outcome.rs: the errors ordered through a map from id to first position, `sort_by_cached_key`.
- src/maildir/backend.rs: every id of a write located from one read of the Maildir (`MaildirClient::locate_many`), each entry kept once.
- No regression test (complexity alone, no timing run); the suite passes.
- CHANGELOG (Fixed).
- Left upstream: io-maildir 0.5's write helpers locate their entry again, one folder read per id written.

## Capabilities moved

None: the behaviour is unchanged.

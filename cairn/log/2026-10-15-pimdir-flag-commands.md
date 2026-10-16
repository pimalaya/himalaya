---
cairn: log
change: pimdir-flag-commands
landed: 2026-10-15
---

# pimdir stages any flag, and a refused flag change stages nothing

Asked in [#790]. A pimdir store carries the keywords its sources read, and nothing could write one back: the shared `flag` commands take the four flags every backend handles, and the pimdir namespace had no flag command. `pimdir flag list|add|set|remove` now read and stage any flag by its raw spelling, as `imap store` sends it. A registered flag is taken in its canonical spelling only, so `seen` is refused rather than staged as a keyword the reader would show as read; `\Deleted`, `\Recent`, a backslash before any other word, non-ASCII characters and the characters IMAP forbids in a flag are refused, as is a write naming no message or none that exists. The writes print the queue row of each message, as the other pimdir writes do, and `list` prints `null` for flags the sync engine never read.

Staging moved into `stage_flags`, which the shared `store_flags` now wraps. It checks every message the mailbox holds against the sources' declarations before queueing the first row, and skips an id the mailbox does not hold, so the shared commands report how many they found as the commands spec has required since [#777]. Before, a multi-id call queued the first messages and then failed on an unknown id or a refused change, leaving them staged. Each row is still its own transaction, so an enqueue failing after the check can leave earlier rows queued.

Adds and removes now match flags as flags rather than as strings, on the shared commands too: a remove of `\Seen` left a `\seen` the store held, and an add put `\Seen` beside it.

Settled while landing, beyond the proposal as reviewed: the staging function is `stage_flags` rather than a changed `store_flags`, the validation refuses backslash words, IMAP specials and non-ASCII, a write naming no message is refused, flags are matched by flag (a `set` deduplicating as an `add` does), `list` prints `null` for an unread set, and an unknown id is skipped rather than failing the call, matching the requirement [#777] landed the same day. The delta and the proposal were edited to say so, and the delta's two requirements reordered so each keeps its own scenarios.

Asked on [#793]: on a message whose flags were never read, an add or a remove is now refused, saying the flags were never synced, on the shared commands too. Before, an add staged the given flags as the whole set and a remove an empty one, replacing whatever the server held; `pimdir flag remove` would have reached that directly. A `set` still replaces the set, which is what it means. The upstream test that asserted the old staging now asserts the refusal, and two tests that staged flags on seeded mails pick ones whose flags were read.

Verified by the test suite, which covers a keyword round trip, removal, replacement, flags held in another spelling, an unread set, a `none` and a partial declaration, a call refused part-way, and an unknown id skipped; the part-way and spelling tests fail on the previous staging. Run by hand against a store a neverest 0.3.0 sync filled from IMAP: `list` showed the stored and the staged flags, a keyword was staged as one row and cancelled, and `seen`, `\Deleted`, a value with a space and an unknown mailbox were refused. That neverest declares no capabilities, so no declaration gated the keyword there; the gate was exercised by the tests only.

Capabilities moved: **backends** (pimdir stages flags by their raw spelling, pimdir stages a flag change on every message or none, pimdir writes name their queue row, pimdir is a reader and a producer, never the owner).

[#777]: https://github.com/pimalaya/himalaya/issues/777
[#790]: https://github.com/pimalaya/himalaya/issues/790
[#793]: https://github.com/pimalaya/himalaya/pull/793

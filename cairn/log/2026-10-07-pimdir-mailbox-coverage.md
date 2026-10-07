---
cairn: log
change: pimdir-mailbox-coverage
landed: 2026-10-07
---

# pimdir lists each mailbox with its coverage

io-pimdir is pinned to `35a1c3f` (pimdir scoped-mail-sync): no probe tier, no `probes` table, so Himalaya reads the stores an owner on that io-pimdir reconciles. `himalaya pimdir mailbox list` lists each mailbox of the account with `total`, `unread`, `coverage` (`{since, until, at}`, `null` until every source closed a round on it) and `round` (`{since, until, startedAt}`, `null` when none is under way); its table shows them as days in COVERED and SYNCING. The shared `mailbox list` is unchanged. An envelope's `has-attachment` is the stored attachment mark: `true`, `false`, or `null` on a row holding none.

Capabilities moved: **backends** (pimdir lists each mailbox with its coverage; the envelope's attachment mark as stored).

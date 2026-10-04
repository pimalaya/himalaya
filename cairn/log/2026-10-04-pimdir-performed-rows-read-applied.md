---
cairn: log
change: pimdir-performed-rows-read-applied
landed: 2026-10-04
---

# A sent message reads applied

io-pimdir is pinned to `8c83c04` by git rev, the commit neverest and calendula pin too. Its `acknowledge_action` records a receipt when the sync engine acknowledges an intent it performed, so `himalaya pimdir queue show <ROW>` reads a message sent or a mailbox created as `applied`, without `seq`, where it read `unknown` like a cancelled row. Nothing changes in himalaya's code but documentation; a unit test acknowledges a sent row as neverest does and reads it applied.

Capabilities moved: **backends** (`pimdir queue show` names performed rows).

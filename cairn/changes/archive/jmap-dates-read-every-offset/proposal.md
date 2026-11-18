---
cairn: change
id: jmap-dates-read-every-offset
status: landed
created: 2026-11-18
---

# JMAP dates read every offset

## Why

Finding HIM-6 of the family's cli-family-analysis-2026-11-15 plan: on the chain of `and` a JMAP `date` or `after` clause sent `receivedAt >= UTC midnight` of the day, while the residual compares the day the `Date:` writes in its own offset. `Date: Fri, 15 May 2026 00:30:00 +0900` is received on the 14th in UTC, so `envelope search date 2026-05-15` never saw it, where IMAP, Maildir and mbox list it.

## What

The wire bound of a residual-checked day is its first instant in any offset, 10:00 UTC the day before (UTC+14:00). Under `or` and `not` nothing changes.

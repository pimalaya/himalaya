---
cairn: change
id: a-wrong-weekday-keeps-its-date
status: landed
created: 2026-11-15
---

# A wrong weekday keeps its date

## Why

Task H21 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (gmail-google.md, H-2): a Gmail envelope dated `Mon, 15 Nov 2026 11:00:00 +0000` (a Sunday) printed `"date":null` and sorted first by date, chrono refusing a weekday that disagrees with the date, where IMAP retried without it and Maildir, mbox and the read view read the date.

## What

The weekday-tolerant parser IMAP had (`parse_rfc2822_date`, retrying without the day-of-week) moves to src/email/envelope.rs, and Gmail reads its metadata `Date` through it.

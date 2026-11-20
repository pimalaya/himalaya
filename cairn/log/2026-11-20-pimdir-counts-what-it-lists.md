---
cairn: log
change: pimdir-counts-what-it-lists
landed: 2026-11-20
---

# pimdir counts what it lists

`mailbox list --counts` and `pimdir mailbox list` on a pimdir account mixed two readings of the store: `total` came from io-pimdir's `count_items`, which folds the queue in, while `unread` came from `count_unread`, which reads committed rows only. After `flag add seen 5` and `message move -t archive 5`, INBOX printed `4/4` while `envelope list` showed 2 unread, and Archive `0/3` while it listed message 5 unread. Found by the pimdir backend test run of 2026-11-20.

## What landed

- src/pimdir/backend.rs: `count_unread` keeps the store's unread count for a mailbox whose queue changes nothing it shows (`PimdirReader::overlay_quiet`, the check `envelope search` already makes before trusting the chip), and otherwise walks the mailbox through the overlaying reader, counting the items without `\Seen` as the chip reads them; `list_mailboxes` and `list_covered_mailboxes` both count through it.
- cairn/spec/backends.md: *Mailbox counts are asked for* says pimdir counts as the overlaying reader lists, with a scenario; *pimdir lists each mailbox with its coverage* counts as the unified listing does.

## Checks

`mailbox_counts_read_the_queue_as_the_listing_does` stages a `\Seen` add and a move and compares both listings' counts with `envelope list`; it failed before the fix (INBOX 35 unread counted, 31 listed; Archive 12 counted, 14 listed).

## Left

The store's unread count reads `\Seen` by its exact bytes, so a message the store holds as `\seen` counts unread while `envelope list` shows it read, on a quiet mailbox and a busy one alike. The test seeds one spelling; aligning the two is io-pimdir's question.

---
cairn: change
id: pimdir-paged-envelope-reads
status: landed
created: 2026-10-07
---

# pimdir reads only the page an envelope listing needs

## Why

`envelope list` and `envelope search` on a pimdir account read every item of the mailbox from the store, built every envelope, filtered and sorted them in memory, then sliced one page. Asking for 50 mails of a 100,000-mail mailbox read 100,000 rows. A reader such as MOA lists and searches a mailbox every few seconds, so the cost grew with the store rather than with the page.

## What

- `envelope list` with a page size reads the store newest first by keyset pages and stops at the end of the page asked for: page N of size S reads N × S rows, never the whole mailbox. Without a page size it reads the mailbox, as before.
- `envelope search` reads newest first too and stops once the page is full when the order is the default one (date descending). It pushes down what the store's own order and readers express exactly:
  - date clauses at the top of the query (`date`, `after`, `not after`): the store's mail sort key is the `Date` (pimdir STORAGE §9.3, Annex A.1), so they bound the keyset walk, which starts below the upper bound and stops below the lower one;
  - `not flag seen` at the top of the query: io-pimdir's `list_mail_page_filtered` with the unread chip, when no queued action changes what the mailbox shows (the chip reads committed rows only, while Himalaya's reads overlay the queue).
  Every other clause (from, to, subject, body, other flags, `or`, other negations) and every other sort is evaluated in memory on the rows read, as before.
- Output is unchanged: the same envelopes, in the same order, for every query. A row whose sort key is not its summary date, which the store's ordering contract rules out, makes the search fall back to the whole-mailbox read.

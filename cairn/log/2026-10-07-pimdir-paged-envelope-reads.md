---
cairn: log
change: pimdir-paged-envelope-reads
landed: 2026-10-07
---

# pimdir reads only the page an envelope listing needs

`envelope list` on a pimdir account reads the mailbox newest first by keyset pages up to the end of the page asked for, rather than every item: page N of size S reads N × S items. `envelope search` reads newest first and stops once the page is full in the default order (date descending); its top-level date clauses (`date`, `after`, `not after`) bound the walk on the sort key, the summary date, and `not flag seen` reads through io-pimdir's unread chip (`list_mail_page_filtered`) when no queued action touches the mailbox. Every clause is still matched in memory and every other sort applied to the rows read, so output is unchanged; a row whose sort key is not its summary date sends the search back to the whole-mailbox read. On 50,000 mails, `envelope list -s 50` went from 3.65 s to 28 ms (release build).

Sender, recipient, subject and body clauses, flags other than unread, and other sorts are not pushed down: io-pimdir's `search_mail` matches the subject or the first sender together, with ASCII case folding, and its unread chip reads `\Seen` in that one spelling, neither matching Himalaya's query exactly.

Capabilities moved: **backends** (pimdir reads only the page an envelope listing needs).

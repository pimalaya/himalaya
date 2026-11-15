---
cairn: change
change: pimdir-paged-envelope-reads
---

# Delta

## ADDED Requirements

### Requirement: pimdir reads only the page an envelope listing needs
`envelope list` on a pimdir account with a page size SHALL read the mailbox newest first by keyset pages and stop at the end of the page asked for: page N of size S reads at most N × S items, overlaid as every read is. Without a page size it SHALL read the whole mailbox.

`envelope search` SHALL read newest first and, when the order is the default one (date descending, or a sort that is `date desc` alone), stop once the page asked for is full. Date clauses at the top of the query (`date`, `after`, `not after`, joined by `and`) SHALL bound the walk on the store's sort key, the mail sort key being the summary date (pimdir STORAGE §9.3, Annex A.1). `not flag seen` at the top of the query SHALL read through io-pimdir's unread chip (`list_mail_page_filtered`) when no queued `set-flags`, `update`, `remove`, `move` or `copy` touches the mailbox, the chip reading committed rows only. Every clause SHALL still be evaluated on the envelopes read, and every other sort applied to them, so a search returns the same envelopes in the same order as a whole-mailbox read. A row read whose sort key is not its summary date in the store's canonical form SHALL make the search read the whole mailbox instead.

#### Scenario: The first page of a large mailbox
- GIVEN a pimdir mailbox holding 50,000 mails
- WHEN `himalaya envelope list -s 50` runs
- THEN the store reads the 50 newest items, and one more per queued removal, rather than the 50,000

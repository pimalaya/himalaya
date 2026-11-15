---
cairn: delta
change: a-body-clause-reads-the-local-body
---

## MODIFIED Requirements

### Requirement: Per-backend translation
Each backend SHALL translate the shared query into its native surface. IMAP translates to `UID SEARCH` / `UID SORT` keys, with a client-side sort fallback when the server lacks `SORT`. JMAP translates to a `JmapFilter` and comparator list scoped to the mailbox id, over-approximating date clauses and re-checking them client-side. Maildir, m2dir and mbox list then evaluate a shared client-side matcher (`matches_filter` plus `sort_envelopes`) over the fetched envelopes, reusing the read bytes for body clauses. pimdir evaluates the same matcher over envelopes built from the stored summaries, reading a message's local body only when the filter holds a body clause; a message whose body is not local matches no body clause. Gmail translates to a `q` value. Graph translates to a KQL `$search` and sorts the page locally, Graph refusing `$skip` and `$orderby` beside it.

#### Scenario: A pimdir body clause
- GIVEN a pimdir mailbox holding two local bodies, one containing `attached`, and one message whose body is not local
- WHEN `envelope list -m imap/INBOX body attached` runs
- THEN only the message whose local body holds `attached` is listed, and `not body attached` lists the two others

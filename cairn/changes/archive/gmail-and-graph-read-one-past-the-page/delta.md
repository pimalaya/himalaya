---
cairn: delta
change: gmail-and-graph-read-one-past-the-page
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Listings resume from their cursor
`envelope list` and `envelope search` SHALL print `nextPage` beside `envelopes`: the next page number, as a string, while a page follows the one printed, `null` once the page reaches the end of the listing, an exactly full last page included, and SHALL take it back through `--page-token`, which wins over `--page`. Whether a page follows SHALL come from the listing's own exchanges: one does when the page ends before the count the backend holds (the messages Maildir and mbox list or match, the `EXISTS` of IMAP's `SELECT` for a listing and the UIDs its `SORT` answers for a search, the `total` of JMAP's `Email/query`, the items pimdir counts for a listing), when pimdir's search finds one hit past the page, when the listing of Gmail or Graph holds one message past the page, which the page asks along (`maxResults` or `$top` one longer, a later Gmail page and a Graph search reading the pages before it in as few calls as the largest page allows, the next page token or link followed only while the listing runs short of that message), both answering a next page token or link after an exactly full last page too, and, on a JMAP server computing no `total`, when the page comes back full. Pages SHALL be numbered from 1: `--page 0` and a page token `0` are refused, `--page-size 0` listing the whole mailbox. `msgraph messages list`, `msgraph mail-folders list` and `msgraph mail-folders child-folders` SHALL print the `$skip` of Graph's next link as `nextPage` and take it back through `--page-token`, a next link leading back to the page asked (`--top 0`) giving no next page; a next link carrying no `$skip` (a `$search` page) SHALL be printed whole, in the text of `msgraph messages list` as `Next link:` rather than `Next page:`, and refused by `--page-token`. `msgraph messages list --search` SHALL quote its value for Graph, its double quotes escaped, unless it opens on a double quote, and `--count` SHALL print Graph's `@odata.count` as `count` (`Count:` in text), `null` without it or beside `--search`, Graph ignoring `$count` there. The Gmail listings keep Gmail's page token. `jmap email query`, `jmap mailbox query` and `jmap submission query` SHALL print `nextPage` beside their collection the same way, a page following while the query's `total` lies past it, or, on a server computing none, while it comes back full, and SHALL take it back through `--page-token`, a token that is no page number from 1 being refused; `jmap email get`, `jmap mailbox get`, `jmap submission get` and `jmap submission create`, printing the same shape, SHALL print `nextPage` as `null`.

#### Scenario: A second page
- GIVEN a mailbox holding two messages
- WHEN `himalaya --json envelope list -s 1` runs, then `himalaya --json envelope list -s 1 --page-token 2`
- THEN the first prints one envelope and `"nextPage":"2"`, the second the other envelope and `"nextPage":null`

#### Scenario: An exactly full last page
- GIVEN a mailbox holding two messages
- WHEN `himalaya --json envelope list -s 2` runs
- THEN it prints both envelopes, `"nextPage":null` and `"total":2`, and its text prints `Page:` `1, envelopes 1 to 2, the last` with no `Next page:` line

#### Scenario: A JMAP query reaching its total
- GIVEN a JMAP account holding two mailboxes, its server computing `total`
- WHEN `himalaya --json jmap mailbox query -s 2` runs
- THEN it prints both mailboxes and `"nextPage":null`, and its text prints `Page:` `1, mailboxes 1 to 2, the last` with no `Next page:` line

## REMOVED Requirements

None.

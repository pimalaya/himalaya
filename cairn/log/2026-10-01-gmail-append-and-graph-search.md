---
cairn: log
change: gmail-append-and-graph-search
landed: 2026-10-01
---

# Gmail appends, Graph searches

`message add` works on Gmail. It goes through `messages.insert`, with the mailbox and the flags as labels (an unseen message carries `UNREAD`) and `internalDateSource=dateHeader`. As with an IMAP `APPEND`, nothing is sent and no filter runs.

`envelope search` works on Microsoft Graph. The filter becomes a KQL `$search` (`from:`, `to:`, `subject:`, `body:`, `sent:` and `sent>`, with `AND`/`OR`/`NOT`). A flag clause is refused by name, since Graph takes no `$filter` beside `$search`. `$skip` is refused too, so a page is reached through the paging links. Graph answers in relevance order, so each page is sorted locally, newest first unless the query sorts.

Graph still has no `add_message`: a MIME message it creates stays a draft.

Capabilities moved: **backends** (Gmail append), **search** (Graph translation; the stale "search-less backends" requirement, already false for Gmail, is gone).

Not verified against a live account.

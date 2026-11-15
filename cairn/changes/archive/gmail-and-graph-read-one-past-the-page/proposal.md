---
cairn: change
id: gmail-and-graph-read-one-past-the-page
status: landed
created: 2026-11-15
---

# Gmail and Graph read one message past the page

## Why

Task H20 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (gmail-google.md and msgraph-microsoft.md, H-3): H1 named no next page after an exactly full last page, but Gmail and Graph answer a next page token or a next link there too, so their `nextPage` still pointed at an empty page (Gmail `-s 6` over 6 messages printed `"nextPage":"2"`, Graph `-s 5` over 5).

## What

Gmail and Graph ask each page one message longer and drop the extra, a page following when it came back, at no extra request. Graph's `envelope list` asks `$top` one longer at its `$skip`; its search, paged by links, and Gmail's listings, paged by token, read up to one message past the page, the first call asking the pages before it and that message at once (`maxResults` up to 500, `$top` up to 1000), a token or link followed only while the listing runs short of it. A later Gmail page so costs one `messages.list` where it cost one per page before it. Past the largest page, Graph's `envelope list` falls back on its next link.

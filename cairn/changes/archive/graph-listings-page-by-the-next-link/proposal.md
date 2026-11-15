---
cairn: change
id: graph-listings-page-by-the-next-link
status: landed
created: 2026-11-15
---

# Graph listings page by the next link

## Why

Ruling D18 of the family's cli-alignment-2026-11-15 plan (report X3, row M1, and the paging part of schema review A6), now pimalaya-cli's cairn/spec/output.md (*Paging resumes*, *Protocol commands are raw-faithful*): every protocol listing that can stop early prints `nextPage`, the provider's cursor as a string as the provider gives it, `null` once there is none, and takes it back as is through `--page-token`. Himalaya's Microsoft Graph listings printed the `$skip` they worked out from Graph's next link instead, and printed a `$search` page's link whole, which `--page-token` then refused: a search could not be resumed.

## What

- `msgraph messages list`, `msgraph mail-folders list` and `msgraph mail-folders child-folders` print Graph's `@odata.nextLink` as `nextPage`, as Graph gives it, `$search` pages included, `null` without one or when it leads back to the page asked (`--top 0`); `--page-token` follows it as is through io-msgraph's `page_get`, the query it carries replacing the other options, and a link off Graph's origin is refused before any request, the bearer token never leaving it.
- `msgraph messages list` text prints `Next page: --page-token <link>`, the `Next link:` line gone.
- `msgraph attachments list` follows every next link, so it never stops early and prints no cursor.
- Checked, unchanged: the Gmail listings that page (`gmail messages list`, `threads list`, `drafts list`, `history list`) print Gmail's `nextPageToken` as `nextPage` and take it through `--page-token`, Gmail's other lists holding no cursor; the JMAP queries (`jmap email query`, `jmap mailbox query`, `jmap submission query`) print, as a string, the page number they resume from, JMAP giving no cursor; IMAP, ManageSieve and the local stores' protocol lists answer whole.

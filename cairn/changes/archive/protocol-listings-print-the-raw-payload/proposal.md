---
cairn: change
id: protocol-listings-print-the-raw-payload
status: landed
created: 2026-11-15
---

# Protocol listings print the raw payload

## Why

Line X9 of the family's cli-alignment-2026-11-15 plan, round X10, before 3.0. Ruling D18 (pimalaya-cli cairn/spec/output.md, *Protocol commands are raw-faithful* and *Paging resumes*) keeps a protocol listing's payload as the provider sent it, the cursor alone lifted into `nextPage`; Himalaya's Gmail, Graph and JMAP listings still re-serialized typed rows that dropped fields (Gmail's `resultSizeEstimate`, a Graph property io-msgraph does not model, the JMAP query's `total` and `queryState`). Ruling D25: every app refuses the query options beside `--page-token` (exit 2), as Cardamum and Calendula do with clap conflicts, where Himalaya ignored them.

## What

- X9a: `msgraph messages list` refuses `--folder`, `--top`, `--skip`, `--filter`, `--search`, `--orderby`, `--select` and `--count` beside `--page-token`; `msgraph mail-folders list` and `child-folders` refuse `--top`, `--skip`, `--select` and `--hidden`, and `child-folders` takes its parent id or a token, not both. The Gmail, JMAP and pimdir tokens carry no query (Gmail's page token wants the same query again; JMAP and pimdir resume by page number), so their options stay.
- X9b, the shape Cardamum prints through pimalaya-cli's `Paginated` (`{<collection>: [<raw item>...], <the provider's other keys>, nextPage}`):
  - Gmail `messages list`, `threads list`, `drafts list`, `history list`: Gmail's page as sent, through a `GmailSend<Map>` (`GmailClient::raw_list`), `nextPageToken` lifted.
  - Graph `messages list` (`@odata.count` as Graph spells it, where `count` stood), `mail-folders list|child-folders`, `attachments list`: raw items through io-msgraph's `page_get`.
  - JMAP `email query|get`, `mailbox query|get`, `submission query|get`: the query chained into its get in one raw request (src/jmap/raw.rs), the get's `list` as sent, beside the query's `queryState`, `canCalculateChanges`, `position`, `total` and the get's `state`; `notFound` stays in `errors`.
- Himalaya keeps its own text layout (the `Next page:` line in the header block), so the outputs keep their types rather than wrap `Paginated`, whose JSON they match.

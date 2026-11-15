---
cairn: log
change: protocol-listings-print-the-raw-payload
landed: 2026-11-15
---

# Protocol listings print the raw payload

Line X9 (X9a, X9b) of the family's cli-alignment-2026-11-15 plan: rulings D18 and D25 in Himalaya.

## What landed

- X9a: the Graph listings refuse their query options beside `--page-token` (clap conflicts, exit 2); `child-folders` takes its parent id or a token.
- Gmail: `GmailClient::raw_list` reads a listing page as a JSON map, `nextPageToken` lifted, the collection `[]` when Gmail leaves it out; `gmail messages|threads|drafts|history list` print it, `resultSizeEstimate` and `historyId` kept, a history record as Gmail sent it (`messagesAddedDetails` gone).
- Graph: `raw_page` builds the first page's link as io-msgraph does and reads it through `page_get::<Value>`; `msgraph messages list` (`@odata.count` in place of `count`), `mail-folders list|child-folders` and `attachments list` print each item as sent.
- JMAP: src/jmap/raw.rs sends `Foo/query` chained into `Foo/get`, or `Foo/get` alone, through `send_raw`, a method `error` failing with io-jmap's `JmapMethodError`; the email, mailbox and submission queries and gets print the `list` as sent beside the query's and get's keys, `notFound` in `errors`; an object no longer turns into a `message-unreadable` or `mailbox-unreadable` entry there.

## Capabilities moved

- [commands](../spec/commands.md): *Protocol listings print the provider's payload* added; *Listings resume from their cursor* refuses the query options beside a Graph token and keeps `@odata.count`; *A command acting on several items lists what failed* drops the unreadable entries of the JMAP gets; *Gmail history retains added-message details* removed.

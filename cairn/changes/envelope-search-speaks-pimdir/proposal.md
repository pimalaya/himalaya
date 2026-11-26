---
cairn: change
id: envelope-search-speaks-pimdir
status: landed
created: 2026-11-21
---

# Envelope search speaks pimdir

Closed 2026-11-26 as landed (log 2026-11-21): the io-pimdir release replaced the git pin; the index-answered search moved to cairn/changes/envelope-search-reads-the-index.

## Why

Himalaya's `envelope search` has a grammar of its own (chumsky, about 1,400 lines under src/unified/email/search/), while Calendula and Cardamum read the pimdir SEARCH language (§8) through io-pimdir, and Himalaya itself does under `pimdir search`. One language across the family means one help page, one MCP tool description and one set of conformance vectors, and a pimdir account answering `envelope search` from its index nearly 1:1. Before v3 is the time: the change breaks every query a user writes.

It rests on two changes in flight: pimdir cairn/changes/the-query-serves-every-client (`header:`, instants, orders) and io-pimdir cairn/changes/the-query-split-by-domain (the grammar with no dependency, the `mail` matcher, predicate names).

## What

- **The query is §8.** `envelope search <QUERY>...` joins its words and parses them with `PimdirQuery::parse`, a refusal coded `search-invalid` and drawn with ariadne over the query, as Cardamum draws it. `from alice and after 2026-01-01` becomes `from:alice date:2026-01-02..`; `message-id <x>` becomes `header:message-id:<x>`; `5 hours ago` becomes `5h`. The chumsky grammar, `SearchEmailsQuery` and the chumsky dependency go.
- **The order is a flag.** `--sort date|subject|from[:asc|:desc]` (default `date:desc`), mapped onto `PimdirSearchSort` (`subject` is `Title`, `from` is `Sender`). `to` goes: neither the index nor the family orders by recipient.
- **Each backend serves a subset.** The capability options become io-pimdir's predicate names (`envelope.search.from`, `envelope.search.header.message-id`, `envelope.search.date.instant`, `envelope.search.is.unread`, ...), each backend declaring `full`, `partial`, `server` or `none`, a predicate no backend of the account serves refused before connecting under `capability-missing`. Fields outside mail (`kind:` other than `mail`, `status:`, `priority:`, `person:`, `has:phone`, `location:`, ...) are `none` everywhere.
- **Text matches words.** §8.5 matches tokens; IMAP and JMAP servers match substrings, so their text predicates are `partial`, the detail saying a substring server matches more; Gmail and Graph match words.
- **Translations** rewritten over `PimdirQuery`: IMAP (`SEARCH` keys, the instant band kept, `HEADER` for any `header:` name), JMAP (filters, the sent-date residual kept), Gmail (`q`, `rfc822msgid:` for the Message-ID), Graph (KQL, `internetMessageId` for the Message-ID), Maildir and mbox through io-pimdir's `search::mail` matcher instead of src/unified/email/search/eval.rs.
- **pimdir answers from its index** when the store has one (`envelope search` as `pimdir search kind:mail` scoped to the mailbox), and through the `mail` matcher over the summaries otherwise, as today.
- **Docs.** The `QUERY` help, the MCP `envelope_search` description (its clause list), cairn/spec/search.md rewritten around §8, MIGRATION.md's v2 to v3 section with the old-to-new table, the README header block's command (`envelope search --json from:alice date:2026-11-02.. --sort date:desc`), CHANGELOG.

## Cost

About 1,400 lines removed (parsers, query types, the local evaluator), five translators rewritten (IMAP, JMAP, Gmail, Graph, local), the capability table renamed, the search tests rewritten (the parser's go to io-pimdir's vectors). A few days.

## Parallel work

Himalaya starts on the io-pimdir of today, under a `[patch.crates-io]` path to the io-pimdir checkout: the parse, the flag, the capability names, the translations of every predicate §8 already has. `header:`, instants and the sort direction follow as io-pimdir lands them in the same checkout. The patch is replaced by the io-pimdir 0.20.0 release before v3.

## Open

- `--sort` syntax: `date:desc` or `date --desc`? Calendula and Cardamum take `--sort date|name|relevance` with no direction today.
- `envelope list` keeps no query, as now?

---
cairn: change
id: search-message-id
status: landed
created: 2026-11-09
---

# Search by Message-ID

## Why

The `Message-ID:` is the one identifier of a message stable across mailboxes, moves and UIDVALIDITY changes, and the query language had no way to find a message by it ([#797]). `imap search --text` is no substitute: Gmail and Zoho index the id inconsistently for `TEXT`, while `HEADER Message-ID` finds it on both.

A generic `header <name> <pattern>` clause was first proposed ([#811]), but it means one thing on IMAP and the local backends and nothing elsewhere: Gmail has no `header:` operator, only `rfc822msgid:`, an exact match, and Graph and the pimdir summaries know no header but the Message-ID. A clause every backend serves with the same meaning is the id alone, matched whole.

## What

- `message-id <id>`: the message whose `Message-ID:` is the id, given with or without its angle brackets, held bare, compared byte for byte with the envelope's normalised id. Option `envelope.search.message-id`.
- IMAP: `HEADER Message-ID <id>`, a substring match on the server, re-checked whole on the fetched envelopes when the clause sits on the chain of `and` from the root; under `or` and `not` the substring match stands (`partial`).
- Gmail: `rfc822msgid:<id>`, `-rfc822msgid:` negated.
- Graph: a lone `message-id` clause becomes `$filter=internetMessageId eq '<id>'`; beside any other clause it is refused, Graph taking no `$filter` beside `$search` (`partial`).
- Maildir, mbox, pimdir: the shared evaluator compares the envelope's id, no body read.
- JMAP: `none`, io-jmap carrying no `header` condition yet.
- `imap search --header NAME TEXT` sends one `HEADER` key per pair, reaching any header on IMAP.

## Scope / non-goals

A generic header clause stays open in #797 for the backends able to match any header. No change to `pimdir search`'s own language.

[#797]: https://github.com/pimalaya/himalaya/issues/797
[#811]: https://github.com/pimalaya/himalaya/pull/811

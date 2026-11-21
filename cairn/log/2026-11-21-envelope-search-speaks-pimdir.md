---
cairn: log
change: envelope-search-speaks-pimdir
landed: 2026-11-21
---

# Envelope search speaks pimdir

`envelope search` parses pimdir SEARCH §8 through io-pimdir (path patch on the checkout, search-draft-16), its order `--sort`. `SearchEmailsQuery::new` refuses a field outside mail, lowers the query to the filter tree the IMAP, JMAP, Gmail and Graph translators read (a `Text` leaf added, a calendar side to a day, an instant side to the second, a since inclusive as `After*` of the moment before) and the order to the local sort chain. Maildir, mbox and pimdir match with io-pimdir's `search::mail`. The `envelope.search.<option>` names follow `PimdirQueryPredicate::name()`. The chumsky parsers and the local evaluator are gone (about 1,600 lines).

Capabilities moved: search.md (rewritten head: one query language, dates, -h, refusals, the declared options; the Maildir, mbox and pimdir matcher), MIGRATION.md (the v2-to-v3 query table), CHANGELOG.

Left: pimdir answering from its index (and `--sort relevance` there), live runs, the io-pimdir 0.20.0 release in place of the path patch.

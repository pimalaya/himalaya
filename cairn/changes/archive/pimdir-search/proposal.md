---
cairn: change
id: pimdir-search
status: landed
created: 2026-10-30
---

# Full pimdir search through the store's index

## Why

A pimdir store carries a full-text search index (pimdir SEARCH, io-pimdir `PimdirIndex`) with its own query language, threads, counts and coverage. Himalaya searches a pimdir mailbox with the shared in-memory matcher instead, over envelopes built from summaries, reading local bodies only for a body clause since a-body-clause-reads-the-local-body. It cannot rank, thread nor say what it did not cover (pimdir GUIDE §14: "never call a search below it exhaustive"), which the draft a-search-says-what-it-covered asks for and this change absorbs. That draft put the coverage on the shared `envelope list`; a shared output carries no pimdir detail (backends, "Shared outputs carry no backend details"), so the coverage lives on `pimdir search`, and the shared listing keeps none.

The plan was approved on 2026-10-30 (pimdir stable release, plan-himalaya-pimdir-search.md), with these decisions:

1. The full language lives in `himalaya pimdir search` for now. Before the v3 release, the SEARCH.md query becomes the shared API's query system, and the shared clauses translate to it.
2. The index is built and refreshed by the store's owner (neverest, or any other owner such as Android). Himalaya is a query client only: it never creates, rebuilds nor refreshes index.db.
3. When shared clauses are translated (v3), values become prefix terms (`foo*`).
4. Mail only.
5. The `pimdir` feature enables io-pimdir `search`; no per-domain features for now.
6. Search sees the store as of the owner's last refresh; pending operations are not listed in results. Revisit once used in practice.

## What

Under `himalaya pimdir`, scoped to `kind:mail` and the configured account:

- `pimdir search <query>...`: the SEARCH.md grammar, `--sort date|name|relevance`, `--page`, `--page-size`, `--threads`. Table: id (`seq`, usable by `message read`), mailbox, date, from, subject, matched (`body` or `summary`), snippet. JSON: hits, `more`, coverage.
- `pimdir thread <id>`, `pimdir count <query>`, `pimdir tag <query> --add|--remove <flag>` (stages `set-flags` per placement through the queue after the capability gate). The plan spelled the last two `pimdir search count` and `pimdir search tag`; under a `search` taking its query as a positional, `himalaya pimdir search count` would be ambiguous with a search for the word `count`, so the four are siblings, as io-pimdir's own CLI spells them.
- Refusals: plain output renders the SEARCH §8.4 span with himalaya's ariadne setup; under `--json` the failure is Himalaya's error object (commands, "A JSON error carries a stable code") with the code `query-refused` and the §8.4 code, span and expectations in its message. The plan asked for the §8.4 object itself as the JSON error, which the error contract (one failure shape, `himalaya-error`) does not allow without a pimalaya-cli change carrying structured details; open question.
- Coverage line under the table when incomplete: bodiless messages in scope, a range below the mailbox coverage, a round still filling, the index behind the store. A missing or outdated index is a coded error (`index-missing`, `index-outdated`) saying to run the sync engine.
- Tests on a scratch store indexed in the test: body, header, address, flag, date and range hits, threads, tag staging, every refusal code, coverage lines, missing and outdated index errors; the 50k-message benchmark extended with query time.
- Docs: spec/search.md (pimdir native search, the v3 intent), commands.md, backends.md; README pimdir section; CHANGELOG.

## Depends on

- io-pimdir: a query-only `PimdirIndex` open (read-only, no lock, never creates nor rebuilds; a missing index or one at another version or tokenizer is a typed error), the answer saying how far the index lags the store, `collection:` matching ids only, rename reconciliation after a deadline-cut refresh, and a reader call loading mail entries by `seq` list if `PimdirSearchHit` is not enough for the table.
- neverest as the indexer: refresh after each sync and drain under a deadline, a full-build command, `--reset` removing index.db, index errors never failing a sync.

## Later (v3)

The shared `envelope list` query moves to the SEARCH.md language for every backend; each backend translates from it (IMAP, JMAP, Gmail, Graph, maildir), pimdir answering natively, shared clause values as prefix terms.

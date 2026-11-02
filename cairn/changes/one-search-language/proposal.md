---
cairn: change
id: one-search-language
status: landed
created: 2026-11-02
---

# Extend the shared search query

## Why

`envelope search` takes himalaya's own query language: clauses (`date`, `after`, `from`, `to`, `subject`, `body`, `flag`), `and`, `or`, `not` and parentheses with explicit precedence, and `order by`. It is parsed by chumsky in src/email/search into one AST (`SearchEmailsFilterQuery` plus sorters), rendered with ariadne on error, and compiled per backend: IMAP keys, JMAP filters, a Gmail `q`, a Graph KQL `$search`, the shared client-side evaluator for Maildir, m2dir, mbox and pimdir. It is precise and fits a CLI, and it stays: pimdir SEARCH §8 is not adopted, and nothing of this change lives outside himalaya.

What it lacks:

- **Clauses.** No `before` (only `not after`, off by a day as easily as not), no `cc` or `bcc`, no clause over every address, no attachment clause, though JMAP, Gmail, Graph and pimdir answer one.
- **Discoverability.** The syntax is only in the `QUERY` argument's long help, so `-h` shows none of it, and users have said they could not find it.
- **Honest refusals.** A clause a backend cannot compile (every flag on Graph, `answered` and keywords on Gmail) bails with a plain message, no code, and nothing says beforehand which clauses an account answers.
- **Honest matching.** The help promises a case-insensitive substring match, true of IMAP and the local evaluator only: Gmail matches words over the whole message, Graph by KQL.

The capability inventory (cairn/changes/capability-shared-api) settled `envelope.search` as `partial` on Gmail and Graph for these reasons; this change makes the partial precise.

## What

1. **New clauses** in the same grammar and AST:
   - `before <yyyy-mm-dd>`, the dates before that day, beside `date` and `after`;
   - `cc <pattern>` and `bcc <pattern>`, as `from` and `to` read theirs;
   - an any-address clause matching `from`, `to`, `cc` and `bcc` at once (name below);
   - `attachment`, a message holding an attachment, valueless, as the envelope's `hasAttachment` says;
   - `flag <name>` already takes any name (`Flag::from_raw`), keywords included; only its documentation changes, listing `seen`, `answered`, `flagged`, `draft` and "or a keyword".
2. **`-h` lists, `--help` explains.** The first paragraph of the `QUERY` doc comment, which clap shows on `-h`, names every clause, operator and sort key, comma separated, and points at `--help`; the paragraphs after it, shown by `--help`, give each clause's syntax, examples and the precedence. The command's long help says how each backend matches.
3. **What a backend answers is a capability.** The registry of cairn/changes/capability-shared-api gets one option per clause and per sort key, in pimdir's grammar of options (one more word): `envelope.search.from`, `envelope.search.flag`, `envelope.search.attachment`, `envelope.search.sort.subject`, and so on, each `full`, `partial` with a detail, or `none`.
4. **A clause a backend does not answer is refused with a code**, never dropped: `search-unsupported`, before any request, the message quoting the clause as the language writes it (`Cannot search `flag answered`: ...`) and saying why. Pointing at it with ariadne, as a parse error does, was dropped: the AST carries no spans, and quoting the clause names it as precisely. A query the grammar refuses keeps the parser's own error, under a code of its own, `search-invalid`.
5. **Matching is declared, not unified.** Each backend's `envelope.search` detail says how it matches text: IMAP and the local evaluator by case-insensitive substring, Gmail by words over the whole message, Graph by KQL, JMAP as its server defines `text` and the header properties (RFC 8621 §4.4.1). Narrowing a server's answer client-side was left out: one backend would match two ways in one query.
6. **The output stays the envelope list**, with no coverage object: the shared output carries no pimdir detail (decided 2026-10-30, cairn/changes/a-search-says-what-it-covered). What a backend cannot do is static and lives in its capability detail, known before the query.
7. **MCP.** `envelope_search` keeps its `query` argument, a string in this language; its description lists the clauses and sort keys the account's backend answers, from the registry.
8. **pimdir** keeps the shared client-side evaluator over the store's summaries and local bodies for `envelope search`, as today; `pimdir search` stays the store's index view with SEARCH's grammar, untouched here.

## Initial capability table

Read from the current translators and adapters; the implementation confirms each cell with a test.

| Clause or sort | IMAP | JMAP | Gmail | Graph | Local | pimdir |
|---|---|---|---|---|---|---|
| `date`, `after`, `before` | F | F | P | F | F | F |
| `from`, `to` | F | F | F | F | F | F |
| `cc`, `bcc` | F | F | F | F | F | F |
| any address | F | F | F | F | F | F |
| `subject` | F | F | F | F | F | F |
| `body` | F | F | P | F | F | P |
| `flag` | D | F | P | none | F | F |
| `attachment` | none | F | F | F | F | F |
| `order by date` | D | F | P | P | F | F |
| `order by from`, `to`, `subject` | D | F | P | P | F | F |

- Dates on Gmail: `after:` and `before:` take a bare date Gmail resolves in a time zone of its own rather than UTC, and the translator widens `date` to the days around it (src/gmail/search.rs); which zone, and whether the widening is re-checked, is to confirm.
- `body` on Gmail is a word over the whole message, headers included; on pimdir a message whose body is not local matches no body clause.
- `flag` on IMAP is a keyword when `PERMANENTFLAGS` allows it; on Gmail only the flags with an operator (`is:read`, `is:starred`, `is:draft`, `is:important`, `in:spam`), `answered` and keywords having none; on Graph every flag clause is refused, KQL having no flag property and Graph taking no `$filter` beside `$search`.
- `attachment` on IMAP has no `SEARCH` key: a `Content-Type: multipart/mixed` header search would be a superset.
- Sorts on Gmail and Graph order the fetched page only, `messages.list` and `$search` answering in their own order; IMAP sorts with SORT (RFC 5256) or the client-side fallback over the page.

## Decided (2026-11-02)

- **The any-address clause is `address`**, a plain word that reads as a CLI argument; `with` was pimdir's, `participant` Graph's.
- **The attachment clause is a bare `attachment`**, valueless, ending where its word does; `has attachment` would have read as a two-word value.
- **`before` is strict**, as `after` is (the evaluator's `date > target`, IMAP's `SENTSINCE` bumped a day), so `after d and before e` is the days strictly between; IMAP's `SENTBEFORE` is strict already.
- **Gmail dates are UTC days in seconds** (decided with the registry): Gmail reads a bare `yyyy/mm/dd` as midnight PST, so `after d` kept day d and the widened `date d` two days; each bound is Unix seconds of the UTC day, `after:` one second early so a message on the boundary is kept whether Gmail compares strictly or not. Gmail still compares its received date, hence `partial`.

## Found while implementing

- **A JMAP date clause under `not` or `or` was checked as if it stood alone.** Its client-side re-check (the sent date against the clause) was ANDed onto the whole result wherever the clause sat, so `not after 2026-01-01` listed nothing and `date 2026-01-01 or subject x` only that day's mail; the regression test fails on the code before this change. The re-check now runs only on the chain of `and` from the root, and a date clause under `or` or `not` reads JMAP's received date alone, which the capability detail says.

## Not in scope

- pimdir SEARCH §8 as the shared grammar, and any change to `pimdir search`.
- Relevance, snippets, threads and coverage outside pimdir.
- Filter flags (`--from`, `--subject`) beside the query: one interface, the query.

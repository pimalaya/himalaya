---
cairn: log
change: one-search-language
landed: 2026-11-02
---

# Extend the shared search query

The `envelope search` query keeps himalaya's own chumsky grammar (pimdir SEARCH §8 was considered and set aside, as were filter flags beside the query) and grows five clauses: `before` (strict, as `after`), `cc`, `bcc`, `address` over the `From`, `To`, `Cc` and `Bcc` headers, and a valueless `attachment`. The AST (`SearchEmailsFilterQuery`) gained the five variants and a `Display` that writes a filter back in the language, round-tripped by a test.

Each backend compiles them: IMAP to `SENTBEFORE`, `CC`, `BCC` and an `OR` of the four for `address`, refusing `attachment`, SEARCH having no key for it; JMAP to its `cc`, `bcc` and `hasAttachment` conditions; Gmail to `before:`, `cc:`, `bcc:`, a brace group and `has:attachment`; Graph to `sent<`, `cc:`, `bcc:`, `participants:` and `hasAttachments:true` (property names checked against the Graph `$search` documentation). The shared evaluator takes the copy recipients as `EnvelopeCopies`: pimdir passes its summary's, the local backends none, the evaluator then reading the `Cc:` and `Bcc:` headers off the raw message only when a clause needs them. `attachment` reads the envelope's mark, else the raw message's parts.

Two codes join the table: `search-invalid` for a query that does not parse, and `search-unsupported` for a clause the backend cannot search, its message quoting the clause as written. Gmail's flag refusals and Graph's flag refusal, plain messages before, now carry it. The ariadne span planned for the refusal was dropped: the AST carries no spans. `search-unsupported` and its helper are gated on the three backends that raise it, so a build without them warns about neither.

`envelope search -h` lists every clause, operator and the sort on one line and points at `--help`, which gives the grammar, examples and how each backend matches text, where it used to promise a case-insensitive substring everywhere. The Graph translator's header said the same; it now says KQL.

Found while implementing: a JMAP date clause under `not` or `or` was re-checked client-side as if it stood alone, the residual being ANDed onto the whole result, so `not after 2026-01-01` listed nothing. The test `a_negated_date_clause_is_not_rechecked_as_a_positive_one` fails on the code before this change (run on a `git archive` copy of HEAD) and passes after it: the re-check now runs on the root's `and` chain only, and a date clause elsewhere reads JMAP's received date.

Left in the change: the capability options `envelope.search.*` and the MCP description built from them, which wait for the capability registry (cairn/changes/capability-shared-api), and Gmail's date zone.

Verified: 389 tests pass with all features, among them tests/envelope_search.rs on a Maildir; clippy clean on all features; the maildir, imap+smtp, jmap, gmail, msgraph and pimdir builds compile, the msgraph-only build keeping its earlier warnings about the unused evaluator.

Spec: search (One query language and Per-backend translation modified; The query's clauses are listed on -h, A query that does not parse is refused with a code, A clause a backend does not answer is refused with a code added; Graph flag clauses modified, with its code); commands (`search-invalid` and `search-unsupported` in the code table).

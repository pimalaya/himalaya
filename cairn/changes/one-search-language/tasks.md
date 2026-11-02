---
cairn: tasks
change: one-search-language
---

# Tasks

Landed on 2026-11-02 (cairn/log/2026-11-02-one-search-language.md), items 3 and 7 with the capability registry (cairn/log/2026-11-02-capability-shared-api.md).

## Decisions

- [x] The any-address clause's name: `address`
- [x] The attachment clause's spelling: a bare `attachment`
- [x] `before` bounds: strict, as `after`

## himalaya (this repository)

- [x] Grammar and AST: `before`, `cc`, `bcc`, `address`, `attachment`; parser tests, and a `Display` rendering a filter back into the language
- [x] Compile the new clauses per backend: IMAP keys, JMAP filter, Gmail `q`, Graph KQL, the shared evaluator (copies from the raw headers, or the pimdir summary)
- [x] `search-unsupported` for a clause a backend cannot compile, before any request, quoting the clause; `search-invalid` for a parse error; both in the code table
- [x] JMAP: the sent-date re-check only on the root's `and` chain (fixes `not after` and `date ... or ...`)
- [x] `-h`: the `QUERY` doc comment's first paragraph lists clauses, operators and the sort, pointing at `--help`; `--help` keeps the syntax, examples and precedence, and says how each backend matches
- [x] Tests: translator tests for IMAP, Gmail, Graph and JMAP; the evaluator; tests/envelope_search.rs on a Maildir; one test per refusal
- [x] CHANGELOG; README; fold into cairn/spec/search.md and the code table of cairn/spec/commands.md; log
- [x] Capability options `envelope.search.*` in the registry (src/capability.rs), from the initial capability table, JMAP's dates `partial` for what was found while implementing; an option declared `none` refused before connecting; the MCP `envelope_search` description generated from them (tests/mcp.rs)
- [x] Gmail's date zone: a bare date is midnight PST (Gmail API filtering guide), so every bound is now Unix seconds of the UTC day; the `date` widening, never re-checked, is gone (src/gmail/search.rs tests)
- [x] Fold the remaining delta; land

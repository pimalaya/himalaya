---
cairn: spec
capability: search
status: current
---

# Search

Shared envelope search is expressed in one backend-agnostic query language, parsed once and translated per backend. The query type carries a filter tree and a sort list; the parser is built on chumsky and renders its errors through the CLI's ariadne setup.

### Requirement: One query language
The `envelope list` filter and sort arguments SHALL parse into a single `SearchEmailsQuery` (filter plus sort), independent of the backend. The grammar covers the usual header, date and body clauses combined with boolean operators, and a sort list over the standard envelope fields.

### Requirement: Per-backend translation
Each backend SHALL translate the shared query into its native surface. IMAP translates to `UID SEARCH` / `UID SORT` keys, with a client-side sort fallback when the server lacks `SORT`. JMAP translates to a `JmapFilter` and comparator list scoped to the mailbox id, over-approximating date clauses and re-checking them client-side. Maildir, m2dir and mbox list then evaluate a shared client-side matcher (`matches_filter` plus `sort_envelopes`) over the fetched envelopes, reusing the read bytes for body clauses. Gmail translates to a `q` value. Graph translates to a KQL `$search` and sorts the page locally, Graph refusing `$skip` and `$orderby` beside it.

### Requirement: Graph flag clauses
Graph SHALL refuse a filter holding a flag clause, naming the flag: KQL has no flag property and Graph accepts no `$filter` beside `$search`, so dropping the clause would widen the result.

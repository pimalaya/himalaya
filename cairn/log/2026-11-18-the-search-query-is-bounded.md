---
cairn: log
change: the-search-query-is-bounded
landed: 2026-11-18
---

# The search query is bounded

Finding HIM-14 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/email/search/parser.rs: `MAX_NESTING`, `MAX_OPERATORS`, `past_bounds` run before the grammar.
- Tests `nesting_past_the_bound_is_refused` and `operators_past_the_bound_are_refused` (stack overflow before the fix), `nesting_at_the_bound_parses`, `a_query_past_its_bounds_says_which`.
- CHANGELOG (Fixed).

## Capabilities moved

- [search](../spec/search.md): *A query that does not parse is refused with a code*.

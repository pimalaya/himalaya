---
cairn: tasks
change: the-search-query-is-bounded
---

# Tasks

- [x] Tests `nesting_past_the_bound_is_refused` and `operators_past_the_bound_are_refused` (src/email/search/parser.rs) overflowed the stack on the old code; `nesting_at_the_bound_parses`; `a_query_past_its_bounds_says_which` (src/shared/envelope/search.rs) checks the rendered diagnostic.
- [x] `MAX_NESTING`, `MAX_OPERATORS` and `past_bounds`.
- [x] Fold into cairn/spec/search.md; CHANGELOG; the log entry.

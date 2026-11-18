---
cairn: change
id: the-search-query-is-bounded
status: landed
created: 2026-11-18
---

# The search query is bounded

## Why

Finding HIM-14 of the family's cli-family-analysis-2026-11-15 plan, rechecked on the live head 2026-11-18. The search query grammar recursed once per parenthesis without bound, and a chain of `and`, `or` or `not` built a tree as deep as it was long, which the backends then walk: 3,000 nested parentheses, or 100,000 chained operators, aborted the process with a stack overflow (exit 134), killing a `repl` or `mcp` session where the query comes from a model or a script.

## What

- `parse` (src/email/search/parser.rs) runs `past_bounds` first: more than `MAX_NESTING` (32) nested parentheses or `MAX_OPERATORS` (256) bare `and`, `or` and `not` outside quotes is refused as a parse error, a chumsky custom error over the parenthesis or operator past the bound, so it reaches the user as `search-invalid` with the ariadne diagnostic.
- `after 15-11-26` read as year 15, named beside this finding, is not part of this change.

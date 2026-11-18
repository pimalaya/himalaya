---
cairn: change
id: the-gmail-search-keeps-its-groups
status: landed
created: 2026-11-18
---

# The Gmail search keeps its groups

## Why

Finding HIM-2 of the family's cli-family-analysis-2026-11-15 plan, rechecked on the live head 2026-11-18. src/gmail/search.rs wrote a conjunction as `a b` and a disjunction as `{a b}`, and Gmail reads every blank-separated term inside braces as an alternative of its own. `envelope search "from alice and subject invoice or from bob"` went out as `{from:alice subject:invoice from:bob}`, and `"date 2026-09-08 or from alice"` as `{after:... before:... from:alice}`, which matches the whole label, exit 0. IMAP, JMAP and Graph keep the grouping; a wider id set piped into `message delete` is where it hurts.

## What

- `any_of` builds every brace group (the `or` arm of `filter_to_q`, the `and` arm of `not_to_q`), wrapping an alternative in parentheses when `group` finds a blank outside any quote, brace or parenthesis: `{(from:alice subject:invoice) from:bob}`.
- A leaf, a quoted pattern and a nested brace group stay as they were.

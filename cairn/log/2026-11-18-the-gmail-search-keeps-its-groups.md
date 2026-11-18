---
cairn: log
change: the-gmail-search-keeps-its-groups
landed: 2026-11-18
---

# The Gmail search keeps its groups

Finding HIM-2 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/gmail/search.rs: `any_of` and `group`, a brace group wrapping an alternative of more than one term in parentheses; the module doc says so.
- Test `conjunctions_under_disjunctions_keep_their_group`, failing before the fix.
- CHANGELOG (Fixed).

## Capabilities moved

- [search](../spec/search.md): *Per-backend translation*.

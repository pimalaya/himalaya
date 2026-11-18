---
cairn: delta
change: the-gmail-search-keeps-its-groups
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Per-backend translation
Gmail translates to a `q` value, a conjunction as its terms joined by blanks and a disjunction as a brace group, an alternative holding more than one term (a conjunction, the two bounds of a `date` clause, a negated `address` or disjunction) wrapped in parentheses, Gmail reading every blank-separated term inside braces as an alternative of its own; the rest of the paragraph is unchanged.

## REMOVED Requirements

None.

---
cairn: delta
change: the-search-query-is-bounded
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: A query that does not parse is refused with a code
A query nesting parentheses deeper than 32 levels, or holding more than 256 `and`, `or` and `not` (a bare one counting wherever it stands outside quotes), SHALL be refused the same way before the grammar reads it, the diagnostic naming the bound and pointing at the first parenthesis or operator past it.

## REMOVED Requirements

None.

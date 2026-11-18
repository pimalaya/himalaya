---
cairn: delta
change: the-capped-message-is-refused
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: A message too complex is refused whole
A message the MIME parser (mime-rs) stopped splitting at its own entity or nesting cap SHALL be refused the same way, a part it left unsplit hiding what it holds; the `message-too-complex` row of the error code table says so.

## REMOVED Requirements

None.

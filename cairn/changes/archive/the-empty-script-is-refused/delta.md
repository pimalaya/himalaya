---
cairn: delta
change: the-empty-script-is-refused
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: ManageSieve commands
`activate` SHALL refuse an empty name as a usage error, exit 2, before connecting (RFC 5804 reads `SETACTIVE ""` as a deactivation).

## REMOVED Requirements

None.

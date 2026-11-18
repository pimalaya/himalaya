---
cairn: delta
change: the-installer-ends-on-its-outcome
---

# Delta

## ADDED Requirements

### Requirement: The installer ends on its outcome
(packaging) install.sh SHALL fail on an HTTP error of its download (`curl -fsSLo`) and SHALL end a successful install printing the installed binary's `--version`, exiting 0.

## MODIFIED Requirements

None.

## REMOVED Requirements

None.

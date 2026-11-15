---
cairn: delta
change: mbox-flag-set-needs-a-flag
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: The raw Maildir folder commands guard the store
`maildir create` SHALL refuse every level of NAME the unified mailbox writes refuse (blank, holding a control character, unlistable), and an existing folder; `maildir rename` SHALL refuse the same of its NAME. `maildir rename` SHALL refuse the root, an existing target, and on the fs layout a target whose parent does not exist. `maildir delete` SHALL refuse the root, and removes the folder with every folder below it, nested Maildirs and Maildir++ `.<name>.*` folders included. Each SHALL print the name relative to the root. `maildir flag set` and `mbox flag set` SHALL require at least one `-f`, refusing the command line (exit 2) rather than clear every flag.

## REMOVED Requirements

None.

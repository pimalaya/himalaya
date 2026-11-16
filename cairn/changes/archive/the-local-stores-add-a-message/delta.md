---
cairn: delta
change: the-local-stores-add-a-message
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Local storage backends
`maildir message save` becomes `maildir message add` in its two sentences (flags refused with `--subdir new`; the `--subdir` of `maildir message add|copy|move`), and the paragraph adds: `maildir message add` SHALL deliver a message, `save` staying a hidden alias of it.

### Requirement: mbox backend
The raw `mbox` command SHALL expose create, rename, delete and list of mbox files, message add (`save` a hidden alias of it), copy and move, where it read message save, copy and move.

## REMOVED Requirements

None.

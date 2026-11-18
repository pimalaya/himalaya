---
cairn: delta
change: the-mcp-mbox-stays-in-its-root
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: mbox backend
An absolute path passed as a mailbox SHALL open that file, so any mbox reads without configuring it, except under `himalaya mcp`, which SHALL refuse it, read or write, a tool reaching no file outside `mbox.root`.

### Requirement: A tool reads and writes no local file
An mbox mailbox given as an absolute path SHALL be refused, a tool reaching no mbox outside `mbox.root`.

## REMOVED Requirements

None.

---
cairn: delta
change: local-listings-hold-a-few-messages
---

# Delta

## ADDED Requirements

### Requirement: A local listing holds a few messages at a time
(backends) A Maildir listing or search SHALL read its files 64 at a time; an mbox one SHALL drop each message's bytes before reading the next.

## MODIFIED Requirements

None.

## REMOVED Requirements

None.

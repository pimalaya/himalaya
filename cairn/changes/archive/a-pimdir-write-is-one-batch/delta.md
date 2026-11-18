---
cairn: delta
change: a-pimdir-write-is-one-batch
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: pimdir is a reader and a producer, never the owner
(backends) The rows one flag change, copy, move or delete stages SHALL be appended as one batch, each passing the gate once: an id the gate refuses, or the store failing, leaves none staged, the failure naming the refused id.

## REMOVED Requirements

None.

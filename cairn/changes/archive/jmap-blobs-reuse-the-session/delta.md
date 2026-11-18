---
cairn: delta
change: jmap-blobs-reuse-the-session
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: JMAP download host may differ
(provider-quirks) The JMAP client SHALL connect to the download host with the credential the session resolved and keep that connection for the next blob there; an upload host likewise.

## REMOVED Requirements

None.

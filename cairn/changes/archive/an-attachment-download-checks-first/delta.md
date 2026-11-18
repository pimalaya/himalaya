---
cairn: delta
change: an-attachment-download-checks-first
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Downloaded attachments never replace an existing entry
(commands) `attachment download` SHALL check every part id before it writes a file. A name SHALL be cut to 255 bytes at a character boundary, its suffix and an extension of fewer than 32 bytes kept.

## REMOVED Requirements

None.

---
cairn: delta
change: attachment-exclusive-create
---

## ADDED Requirements

### Requirement: Downloaded attachments never replace an existing entry
Attachment downloads SHALL atomically create a new file, refusing to follow an existing leaf symlink. An occupied name SHALL be suffixed, with exhaustion returning an error rather than overwriting a file.

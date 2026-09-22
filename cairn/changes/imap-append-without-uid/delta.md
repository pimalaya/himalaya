---
cairn: delta
change: imap-append-without-uid
---

## ADDED Requirements

### Requirement: A successful append may have no recoverable id

The shared `add_message` result SHALL carry an optional backend id. An IMAP `APPEND` acknowledged as successful SHALL remain successful when the server omits `APPENDUID` and a fallback search cannot recover the UID, including when the server rewrites the submitted `Message-ID`. A command SHALL report the save without inventing an id, and a requested send SHALL continue. An `APPEND` rejected by the server SHALL remain an error.

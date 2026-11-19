---
cairn: delta
change: a-saved-attachment-name-is-neutralised
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: Downloaded attachments never replace an existing entry
Adds: a file name a message gives SHALL become one path component (pimdir STORAGE §18): `/` and `\`, every control character and every bidirectional formatting character (U+061C, U+200E, U+200F, U+202A to U+202E, U+2066 to U+2069) replaced by `_`, the surrounding whitespace and the leading dots dropped, and a name left empty written `attachment`.

## REMOVED Requirements

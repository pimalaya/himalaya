---
cairn: delta
change: the-mbox-cache-is-private
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: mbox reads go through a cache
The cache directory `<XDG cache>/himalaya/mbox/` SHALL be created and kept at mode 0700, each file written at 0600 whatever the umask, the cache holding the subject, sender, recipients and Message-ID of every message.

## REMOVED Requirements

None.

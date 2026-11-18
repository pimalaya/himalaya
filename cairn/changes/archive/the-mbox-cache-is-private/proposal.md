---
cairn: change
id: the-mbox-cache-is-private
status: landed
created: 2026-11-18
---

# The mbox cache is private

## Why

Finding HIM-10 of the family's cli-family-analysis-2026-11-15 plan, rechecked on the live head 2026-11-18. An mbox at mode 600 listed under umask 022 left `<XDG cache>/himalaya/mbox` at 755 and its hash-named JSON file at 644, holding the subject, sender, recipients and Message-ID of every message, readable by every user of a shared host. The family treats this data as private elsewhere (a readable pimdir store is refused as `store-exposed`). The attachment half named beside the finding (downloads landing in the shared temp directory at 0644) is not part of this change.

## What

- `MboxCache::save` (src/mbox/cache.rs) creates the cache directory and its missing parents at 0700 and tightens it to 0700 when it exists open, and writes each file new at 0600 before renaming it into place (unix; other platforms unchanged).

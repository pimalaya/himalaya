---
cairn: log
change: the-mbox-cache-is-private
landed: 2026-11-18
---

# The mbox cache is private

Finding HIM-10 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/mbox/cache.rs: `create_private_dir` (0700, tightening an open directory) and `write_private` (0600).
- Test `a_cache_is_private_to_its_owner`, failing before the fix.
- CHANGELOG (Fixed).

## Capabilities moved

- [backends](../spec/backends.md): *mbox reads go through a cache*.

---
cairn: log
change: imap-reads-resolve-a-role
landed: 2026-11-18
---

# IMAP reads resolve a role

Finding HIM-7 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/imap/backend.rs `get_message`: `resolve_mailbox_id` first.
- Test `reading_resolves_a_mailbox_role`, failing on 002bf012.
- CHANGELOG (Fixed).

## Capabilities moved

- [backends](../spec/backends.md): *A read names its mailbox as a listing does*, new.

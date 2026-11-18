---
cairn: log
change: an-imap-group-is-no-address
landed: 2026-11-18
---

# An IMAP group is no address

Finding HIM-22 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/imap/utils.rs `to_addresses`, one converter for `envelope list|search` and `imap fetch`.
- Test `an_envelope_group_lists_its_members_alone`, failing on 002bf012.
- CHANGELOG (Fixed): the `from` and `to` lists of those commands lose the marker entries.

## Capabilities moved

- [backends](../spec/backends.md): *An IMAP envelope reads its strings as a header*.

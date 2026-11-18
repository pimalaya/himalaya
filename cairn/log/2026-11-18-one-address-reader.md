---
cairn: log
change: one-address-reader
landed: 2026-11-18
---

# One address reader

Finding HIM-12 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/email/address.rs `parse`; src/gmail/backend.rs and src/shared/message/read.rs read through mime-rs.
- Tests `an_address_header_reads_as_the_other_backends_read_it` and `a_quoted_name_holding_an_address_is_a_name`, failing on 002bf012.
- CHANGELOG (Fixed). Left: the read view's comment and validity rules, for mime-rs.

## Capabilities moved

- [backends](../spec/backends.md): *Addresses are read by one parser*, new.
- [commands](../spec/commands.md): *The read view lists unreadable address entries*.

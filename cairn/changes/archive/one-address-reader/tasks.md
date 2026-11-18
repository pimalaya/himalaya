---
cairn: tasks
change: one-address-reader
---

# Tasks

- [x] src/email/address.rs: `parse`.
- [x] src/gmail/backend.rs: `parse_addresses` through it, the comma split gone.
- [x] src/shared/message/read.rs: `mailbox` through mime-rs, `unquote` removed.
- [x] Tests `an_address_header_reads_as_the_other_backends_read_it` (Gmail) and `a_quoted_name_holding_an_address_is_a_name` (read view), both failing on e440b4e4.
- [x] Fold into cairn/spec/backends.md and commands.md; CHANGELOG.

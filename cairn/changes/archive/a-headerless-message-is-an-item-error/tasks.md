---
cairn: tasks
change: a-headerless-message-is-an-item-error
---

# Tasks

- [x] `envelope::from_raw` answers `None` for bytes holding no header field; `error::headerless_message` builds the entry.
- [x] Maildir `list_envelopes_counted` and `search_envelopes`: the entry beside the unreadable files, first page only (`entry_errors`).
- [x] mbox `envelopes`: the entry for each message holding no header field, cached as `None`; `list_envelopes_counted` and `search_envelopes` answer an `Outcome`; cache version 4.
- [x] Tests: `a_message_holding_no_header_field_is_an_item_error` (tests/local_stores.rs, Maildir and mbox, list and search, JSON and text), `a_file_holding_no_header_field_is_an_item_error_of_the_first_page` (src/maildir/backend.rs).
- [x] Fold into cairn/spec/backends.md and commands.md; CHANGELOG and MIGRATION.md; the log entry.

---
cairn: log
change: a-headerless-message-is-an-item-error
landed: 2026-11-15
---

# A headerless message is an item error

Task H9 of the family's cli-alignment-2026-11-15 plan (finding F13).

Reproduced first, with the binary on a scratch Maildir holding one message, an empty file, 300 random bytes, a line of text and a few binary bytes: `envelope list` printed five rows, four of them `-` in every cell, `"errors":[]`, exit 0. A scratch mbox holding one message, one whose lines hold no header field and one opening on a blank line printed the same rows. pimdir was read, not changed: its listings print the summary the sync engine wrote, never parsing a body.

## What landed

- src/email/envelope.rs: `from_raw` answers `None` for bytes whose header holds no field named with RFC 5322 `ftext` (mime-rs never fails a parse and takes any bytes before a colon for a name: the first run left a file of random bytes listed as a row, one line of it holding a colon).
- src/error.rs: `headerless_message`, the `message-unreadable` entry, "Cannot parse the message: it holds no header field".
- src/maildir/backend.rs: the listing and the search push that entry after the files that cannot be read, on the first page only (`entry_errors`).
- src/mbox/backend.rs, src/mbox/cache.rs: `envelopes` answers the entries beside the envelopes; the cache keeps `None` for such a message, its version raised to 4; `list_envelopes_counted` and `search_envelopes` answer an `Outcome`, the entries on the first page only, `total` counting them.
- Tests: tests/local_stores.rs `a_message_holding_no_header_field_is_an_item_error`, Maildir and mbox, list and search, JSON and text, through the binary, which fails on the old code (rows printed, exit 0); src/maildir/backend.rs `a_file_holding_no_header_field_is_an_item_error_of_the_first_page`.
- CHANGELOG (**BREAKING**, exit 3 where v2 exited 0) and MIGRATION.md.

`message read` of such a file still reads it, its bytes as the body.

## Capabilities moved

- [backends](../spec/backends.md): *Each backend names the items it could not serve* (Maildir, mbox), *mbox reads go through a cache*.
- [commands](../spec/commands.md): *A command acting on several items lists what failed*, a scenario; the `message-unreadable` code row.

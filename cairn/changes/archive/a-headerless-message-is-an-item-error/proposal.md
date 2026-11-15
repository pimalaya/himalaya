---
cairn: change
id: a-headerless-message-is-an-item-error
status: landed
created: 2026-11-15
---

# A headerless message is an item error

## Why

Task H9 of the family's cli-alignment-2026-11-15 plan (finding F13, reported, checked first). Reproduced on a scratch Maildir: an empty file, 300 random bytes, a line of text and a few binary bytes, beside one real message, each printed by `envelope list` as a row of `-` (`subject` empty, `from` and `date` null), `"errors":[]`, exit 0. An mbox whose messages hold no header field printed the same rows. mime-rs parses any bytes, so nothing told such a file from a message. Cardamum (1d047d6, def351c) and Calendula list an unreadable card or event in `errors` alone, exit 3.

## What

`envelope::from_raw` answers no envelope for bytes holding no header field, which no message lacks. Maildir and mbox list and search such a message as an `errors` entry coded `message-unreadable` with its id, never a row, on the first page only, exit 3. The mbox cache remembers a message holding no header field (`null`), its version raised to 4 so an older cache holding an empty envelope for one is rebuilt. pimdir lists the summary its sync engine wrote and never parses a body in a listing, so it is left as it is.

Exit 3 where v2 exited 0: CHANGELOG (**BREAKING**) and MIGRATION.md say so.

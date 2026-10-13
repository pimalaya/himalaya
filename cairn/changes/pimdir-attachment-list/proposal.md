---
cairn: change
id: pimdir-attachment-list
status: landed
created: 2026-10-13
---

# pimdir lists and searches an account's attachments

## Why

io-pimdir `14a8fca` records, as a message body enters a store, a stand-in for each attachment in the account's attachments collection (pimdir STORAGE §14.3, role `attachments`), with its file summary and a reference from the message. A reader such as MOA wants a "Files" view of a mailbox: every attachment of the account, newest first, searchable by name and by sender, each opening its message and its bytes. MOA reads stores only through Himalaya, so Himalaya has to say what the store holds.

## What

- io-pimdir pinned to `14a8fca`; nothing in Himalaya broke.
- `pimdir attachment list`: one page of the account's attachments, newest message first, read with io-pimdir's `list_attachments_by_account` (no body opened), each with the stand-in's id, its name, media type, size and IMAP section, and the id, mailbox, sender and date of its message, the ids being those `envelope list` and `message read` take. Pages follow one another by an opaque token (`--page-token`, `nextPage`), keyset-based so a sync between two pages neither skips nor repeats a file.
- `pimdir attachment search <WORDS>`: the same page, narrowed to the files every word of which appears, ignoring case, in the file name, the sender's address or the sender's name. io-pimdir's `search_files` matches the name only and orders by name, so the search walks the newest-first listing and matches in memory, stopping once the page is full.
- `message parse --section <SECTION>`: a part named by its IMAP section (RFC 3501 §6.4.5), which is what a stand-in records, rather than by the view's part id, which a reader cannot know without parsing the message first. A file's bytes are then `message read --raw` piped to `message parse --section`: no command reads a file by its stand-in id.

No media type filter: a family such as "document" is the reader's to define, and `mediaType` is printed.

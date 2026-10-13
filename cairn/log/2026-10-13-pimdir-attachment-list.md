---
cairn: log
change: pimdir-attachment-list
landed: 2026-10-13
---

# pimdir lists and searches an account's attachments

io-pimdir moves from `07bd316` to `14a8fca`, where a message body entering a store records a stand-in per attachment in the account's attachments collection; nothing in Himalaya broke. `himalaya pimdir attachment list` prints one page of the account's attachments, newest message first, from the store's index alone: each file's id, name, media type, size and IMAP section, and the id, mailbox, sender and date of its message, the ids being those `envelope list` and `message read` take, with a `nextPage` token that `--page-token` takes. `pimdir attachment search <WORDS>` keeps the files every word of which is in the name, the sender's address or the sender's name, ignoring case. `message parse --section <SECTION>` writes a part named by its IMAP section, so a file's bytes are `message read --raw` piped to it.

Capabilities moved: **backends** (pimdir lists and searches the account's attachments), **commands** (`message parse --section`).

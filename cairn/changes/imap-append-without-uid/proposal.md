---
cairn: change
id: imap-append-without-uid
status: landed
created: 2026-09-22
---

# Accept a successful IMAP append without a recoverable UID

## Why

RFC 4315 makes `APPENDUID` a SHOULD, so a server can acknowledge `APPEND` without it, even after advertising UIDPLUS. Himalaya then searches the submitted `Message-ID`, but QQ Exmail rewrites that header and the search finds nothing. The message is saved, yet Himalaya reports a failure and the send half of `message send --save` never runs.

## What

The shared `add_message` returns an optional id. IMAP returns none when an acknowledged append yields no UID, logging why at debug level; JMAP returns none when `Email/import` reports no id, instead of an empty string. Maildir, m2dir and pimdir keep returning an id. `message add` reports the save without an id, `null` in JSON, and save-then-send goes on.

---
cairn: change
id: imap-append-without-uid
status: landed
created: 2026-09-22
---

# Accept a successful IMAP append without a recoverable UID

## Why

An IMAP server can acknowledge `APPEND` while omitting `APPENDUID`, even after advertising UIDPLUS. Himalaya then searches by the submitted `Message-ID`, but providers such as QQ Exmail rewrite that header and make the search return no match. The message has been appended, yet Himalaya reports failure and prevents the send half of `message send --save` from running.

## What

Represent an added message id as optional across the shared backend boundary. IMAP returns no id after a successful append whose UID cannot be recovered, while every backend with an authoritative id keeps returning one. Commands report the successful save without inventing an id, and save-then-send continues to SMTP.

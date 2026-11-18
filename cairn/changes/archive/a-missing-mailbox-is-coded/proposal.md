---
cairn: change
id: a-missing-mailbox-is-coded
status: landed
created: 2026-11-18
---

# A missing mailbox is coded

## Why

The live run of 2026-11-18 found a missing mailbox printing `code: null` on every network backend, against the code table and the send-copy scenario of commands.md: IMAP F4 (Fastmail answers `SELECT`, `APPEND` and `MOVE` of a missing mailbox with a plain `NO`, and src/imap/backend.rs `explain` only wrapped `RENAME` and `DELETE`), JMAP J5, Gmail G6 and Graph M4 (their resolvers bailed uncoded).

## What

- IMAP: one rule for every command addressing a mailbox (`SELECT`, `APPEND`, `COPY`, `MOVE`, `RENAME`, `DELETE`). A `NO [NONEXISTENT]` (RFC 5530 §3) or `NO [TRYCREATE]` (RFC 3501 §6.3.11) is `mailbox-not-found` with no round trip. A plain `NO` costs one `LIST` after it: absent, `mailbox-not-found`; listed, the server's refusal. A success, a `BAD`, or a `NO` with another code sends no `LIST`, its own code standing. This is the least costly sound rule: nothing is paid on success, a coded refusal is believed, and only the ambiguous plain `NO` is checked. A `STATUS` of a listed mailbox refused `NONEXISTENT` is an `errors` entry so coded.
- JMAP, Gmail and Graph: the name, id or role no mailbox, label or folder carries is `mailbox-not-found`, Graph's 404 kept in `sources`.
- The copy a send saves inherits the code, its item error coded.
- Breaking for scripts reading `code`: `null` becomes `mailbox-not-found`.

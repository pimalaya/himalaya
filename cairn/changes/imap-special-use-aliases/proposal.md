---
cairn: change
id: imap-special-use-aliases
status: active
created: 2026-07-25
---

# Read IMAP special-use mailbox roles

## Why
The shared `Mailbox` carries a role (see mailbox-role). The Sent/Drafts/Trash/Junk/Archive roles carry RFC 6154 special-use attributes (`\Sent`, `\Drafts`, ...). A plain LIST advertises them on some servers only; the reliable path is LIST `RETURN (SPECIAL-USE)`, which io-imap cannot issue because upstream imap-codec has no support for the RETURN option (duesee/imap-codec#350).

## What landed (2026-11-12, the-live-campaign-fixes)
The IMAP backend reads the special-use attributes a plain LIST carries, as Fastmail and iCloud send them unasked, onto `Mailbox.role`, beside `INBOX`. `role_mailbox_id` and `resolve_mailbox_id` answer from that listing, so `-m sent` and the trash of `message delete` resolve on IMAP, a `mailbox.alias.<role>` still winning. IMAP's `message.trash` is declared `server`.

## What remains
A server marking its roles only under LIST `RETURN (SPECIAL-USE)` (RFC 6154 lets it) shows none, its roles set through `mailbox.alias.<role>`. Once io-imap can issue the RETURN option, the IMAP backend lists with it and maps the returned attributes the same way.

## Blocked on
imap-codec gaining the extended-LIST RETURN option, then io-imap exposing it on its LIST coroutine.

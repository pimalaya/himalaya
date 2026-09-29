---
cairn: change
id: imap-special-use-aliases
status: active
created: 2026-07-25
---

# Read IMAP special-use mailbox roles

## Why
The shared `Mailbox` carries a role (see mailbox-role), but IMAP marks only the reserved `INBOX`. The Sent/Drafts/Trash/Junk/Archive roles carry RFC 6154 special-use attributes (`\Sent`, `\Drafts`, ...), and Himalaya already parses them (the `imap mailbox list` command renders them). The gap is that a plain LIST only advertises the attributes on some servers; the reliable path is LIST `RETURN (SPECIAL-USE)`, which io-imap cannot issue because upstream imap-codec has no support for the RETURN option (duesee/imap-codec#350).

## What
Once io-imap can issue LIST `RETURN (SPECIAL-USE)`, the IMAP backend lists with it and maps the returned attributes onto `Mailbox.role` through `MailboxRole::parse`, on top of the existing `INBOX`. `role_mailbox_id` then answers from that listing, so `-m sent` and the trash resolve on IMAP too.

## Blocked on
imap-codec gaining the extended-LIST RETURN option, then io-imap exposing it on its LIST coroutine.

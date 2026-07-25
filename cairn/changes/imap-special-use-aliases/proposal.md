---
cairn: change
id: imap-special-use-aliases
status: active
created: 2026-07-25
---

# Discover IMAP special-use mailbox aliases

## Why
The wizard pre-fills `mailbox.alias.*` for every backend, but IMAP is limited to the reserved `INBOX`. The Sent/Drafts/Trash/Junk/Archive roles carry RFC 6154 special-use attributes (`\Sent`, `\Drafts`, ...), and Himalaya already parses them (the `imap mailbox list` command renders them). The gap is that a plain LIST only advertises the attributes on some servers; the reliable path is LIST `RETURN (SPECIAL-USE)`, which io-imap cannot issue because upstream imap-codec has no support for the RETURN option (duesee/imap-codec#350).

## What
Once io-imap can issue LIST `RETURN (SPECIAL-USE)`, map the returned attributes to alias keys in the wizard, mirroring the JMAP role mapping: `\Sent` to sent, `\Drafts` to drafts, `\Trash` to trash, `\Junk` to junk, `\Archive` to archive, on top of the existing `INBOX`. The reused connection from the IMAP test lists the mailboxes; a failed or empty listing keeps just `INBOX`.

## Blocked on
imap-codec gaining the extended-LIST RETURN option, then io-imap exposing it on its LIST coroutine.

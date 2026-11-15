---
cairn: delta
change: an-mbox-inbox-without-a-spool-is-not-found
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: mbox backend
The mbox backend SHALL adapt io-mbox over the full unified operation set. A mailbox is one mbox file: `mbox.root` is walked recursively, the mailbox `a/b` being the file `a/b` below it (`a.sbd/b` with `mbox.thunderbird`), and `mbox.inbox`, when set, is the spool shown as `INBOX` and marked with the inbox role. Without `mbox.inbox`, `INBOX` (any case) SHALL fail naming the key, coded `mailbox-not-found`, unless the root holds a file of that name, and a spool that is a directory SHALL be left out of `mailbox list` and fail `account check` and every use of `INBOX`, naming the key. A message command (listing, search, read, add, flag, copy, move, delete and their raw `mbox` twins) SHALL fail on a missing mbox with ``Mbox mailbox `X` not found``, coded `mailbox-not-found`, and SHALL refuse a directory or a file that does not open on a `From_` line before taking any lock, naming the file. An absolute path passed as a mailbox SHALL open that file, so any mbox reads without configuring it. `mailbox delete` and `mailbox rename` SHALL refuse the spool, however named, from the unified commands, the raw `mbox` commands and MCP alike, as IMAP refuses to delete or rename `INBOX` (RFC 3501 section 6.3.4), a rename onto the spool included, and SHALL refuse a directory, which holds mailboxes but is none. A non-empty file below the root that does not open on a `From_` line SHALL not be listed, and an append to it SHALL be refused. A mailbox name with a level starting with `.` or ending in `.lock`, `.lck` or `.msf` (and `.sbd` with `mbox.thunderbird`, refused as the folder suffix of that layout), which no listing shows, SHALL be refused by create, rename and delete. A name that is empty, holds a NUL or a `.` or `..` level, or a level ending in `.lock` or `.msf`, SHALL be refused by every command taking a mailbox, the reads included, io-mbox resolving it to no file. In the plain layout an mbox file holds no mailbox, so a mailbox created under one SHALL be refused naming `mbox.thunderbird`. In the Thunderbird layout creating a mailbox SHALL create its missing parent mboxes empty, a rename SHALL move the mbox's .sbd directory with it, refusing when the target's exists, and a delete SHALL refuse an mbox holding others, naming them. A `mbox.root` that is no directory SHALL fail `mailbox list` naming it. The raw `mbox` command SHALL expose create, rename, delete and list of mbox files, message save, copy and move, a copy or move printing the ids the messages get in the target, each message once, and the six flag letters of the `Status` and `X-Status` headers.

## REMOVED Requirements

None.

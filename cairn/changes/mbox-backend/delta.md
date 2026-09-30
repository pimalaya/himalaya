---
cairn: change-delta
id: mbox-backend
status: landed
---

## ADDED Requirements

### Requirement: mbox backend
The mbox backend SHALL adapt io-mbox over the full shared operation set. A mailbox is one mbox file: `mbox.root` is walked recursively, the mailbox `a/b` being the file `a/b` below it (`a.sbd/b` with `mbox.thunderbird`), and `mbox.inbox`, when set, is the spool shown as `INBOX` and marked with the inbox role. An absolute path passed as a mailbox SHALL open that file, so any mbox reads without configuring it. The raw `mbox` command SHALL expose create, rename, delete and list of mbox files, message save, copy and move, and the six flag letters of the `Status` and `X-Status` headers.

### Requirement: An mbox message is its content id
The mbox backend SHALL address a message by its io-mbox content id, a hash of the message with the flag and metadata fields left out, so a flag write by Himalaya or any other client keeps it. An id the index no longer holds SHALL fail naming the mailbox. An append SHALL report the id the synced index gives the new message, numbered like any duplicate.

### Requirement: mbox flags interoperate
Shared flags SHALL map onto `Status: R` (seen), `X-Status: A`, `F`, `T` and `D` (answered, flagged, draft, deleted), any other flag going to `X-Keywords`. A message marked seen SHALL be marked old (`O`) too, as mutt writes it, since GNU mail lists a `Status: R` alone as new. A shared `flag set` SHALL keep the `O` a message carries, no shared flag naming it.

### Requirement: mbox writes lock and rewrite in place
Every mbox write SHALL go through io-mbox under the dotlock and the fcntl lock, each one skippable through `mbox.lock.dotlock` and `mbox.lock.fcntl`, waiting `mbox.lock.timeout` seconds for a busy one. Flag changes and removals SHALL rewrite the file in place, the whole set of messages a command names in one rewrite. A move SHALL copy then remove, so an interruption leaves the messages in both files.

### Requirement: mbox reads go through a cache
The mbox backend SHALL keep, per file, the io-mbox scan index and the envelope of every message already parsed, under `<XDG cache>/himalaya/mbox/`. Every operation SHALL sync the index first, so an unchanged file is not scanned and a delivery is scanned alone, and a listing SHALL parse only the messages the cache lacks. The cache SHALL be disposable: a missing, stale or unreadable one is rebuilt, failing to write one is not an error, and a read finding its entry stale SHALL rebuild the index and retry once.

### Requirement: The mbox block
An `mbox` block SHALL name `root`, the directory of mbox files, and MAY name `inbox` (the spool), `format` (`mboxo`, `mboxrd` by default, `mboxcl`, `mboxcl2`, which also decides whether `Content-Length` is trusted on read), `thunderbird` (the `.sbd` layout, off by default) and `lock.dotlock`, `lock.fcntl` (both on by default) and `lock.timeout`.

## MODIFIED Requirements

The backend lists of backends.md (threading pointers, append and search gaps, sending transport, mailbox role), commands.md (protocol-specific APIs, shared client), config.md (multi-account schema, path keys), packaging.md (library and feature lists) and search.md (client-side evaluation) name mbox beside Maildir and m2dir.

## REMOVED Requirements

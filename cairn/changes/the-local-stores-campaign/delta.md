---
cairn: delta
change: the-local-stores-campaign
---

# Delta

The requirements below are folded as they now read in the spec; each entry names its capability and what changed.

## ADDED Requirements

### Requirement: The Maildir block
(config) A `maildir` block names `root`, and may name `keywords.dovecot`, `keywords.header` and `layout` (`fs` by default, or `maildir++`); any other layout fails the load.

### Requirement: The raw Maildir folder commands guard the store
(commands) `maildir create` refuses the levels the unified writes refuse and an existing folder; `maildir rename` refuses the root, an existing target and, on the fs layout, a missing parent; `maildir delete` refuses the root; each prints the name relative to the root; `maildir flag set` requires `-f`.

## MODIFIED Requirements

### Requirement: A Maildir is named relative to the root
(backends) The root lists as `INBOX` with the inbox role, or `.` beside a top-level `inbox` folder; every spelling of a mailbox (name, absolute path, `.` segments, trailing slash) resolves to one id, `..` and outside paths refused; an omitted mailbox reaches a top-level `inbox` folder, else the root; `maildir.layout`; a missing Maildir is coded `mailbox-not-found`; dovecot keyword letters renamed on copy and move; a unified move within one Maildir changes nothing; unreadable files and folders and symlink loops skipped; only a `2,` info section carries flags.

### Requirement: Local storage backends
(backends) `--subdir` takes `cur` or `new` only.

### Requirement: mbox backend
(backends) `INBOX` without a spool, or with a spool that is a directory, fails naming `mbox.inbox`; the spool and directories are never deleted or renamed; files that are no mbox are not listed and refuse appends; hidden and side-file names are refused; the Thunderbird layout creates missing parents; the raw copy and move print the target ids.

### Requirement: An mbox message is its content id
(backends) A read leaves out Thunderbird's `X-Mozilla-Status`, `X-Mozilla-Status2` and `X-Mozilla-Keys`.

### Requirement: mbox flags interoperate
(backends) A flag write keeps `X-Mozilla-Status` in step; a delete or rename removes the `.msf`.

### Requirement: mbox writes lock and rewrite in place
(backends) A busy lock past the timeout fails with `store-busy`, a forbidden dotlock names `mbox.lock.dotlock = false`, and a `Content-Length` is trusted only when it ends on a message boundary.

### Requirement: mbox reads go through a cache
(backends) A listing and a search retry once on a stale entry, as a read does.

### Requirement: Mailbox role
(backends) Maildir marks its inbox; a role alias names a mailbox by id, else by the name of exactly one listed mailbox, on every backend.

### Requirement: The JSON contract is described and versioned
(commands) The code table gains `mailbox-not-found` and `message-not-found`; `store-busy` covers mbox locks.

### Requirement: A command given message ids says how many it found
(commands) None found fails with `message-not-found`; the raw local-store commands refuse the whole set on an unknown id; a move within one Maildir changes nothing; `maildir` and `mbox` `message copy|move` print the target ids.

### Requirement: Unified mailbox writes
(commands) A Maildir++ level holding a dot and the mbox side-file names are refused; the root of a local store is never deleted or renamed; a Maildir holding any directory besides `cur`, `new` and `tmp` is not deleted. Scenario "The root Maildir" added.

### Requirement: What a backend searches is declared
(search) The local evaluator lowercases Unicode text in `subject` and `body` alike.

## REMOVED Requirements

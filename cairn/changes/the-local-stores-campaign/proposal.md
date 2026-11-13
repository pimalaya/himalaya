---
cairn: change
id: the-local-stores-campaign
status: landed
created: 2026-11-13
---

# The local stores campaign

## Why

On 2026-11-13 a test campaign ran Himalaya against Maildir and mbox stores. On Maildir, `mailbox delete .` emptied the whole store; the root listed under its folder basename, which no command took back; a trash named by path, with a trailing slash or through an alias was not recognised, so `message delete` there re-trashed; dovecot keywords changed meaning across mailboxes on copy and move; the raw commands wrote part of a set before failing on an unknown id; one unreadable file failed a whole listing. On mbox, the spool could be deleted or renamed; a wrong `Content-Length` merged messages under one id; a stale index failed listings and searches; a busy lock or a forbidden dotlock failed without saying what to do; files that are no mbox were listed; Thunderbird's status fields leaked into reads and fell out of step with the flags.

## What

- Maildir names the root `INBOX` (or `.` beside a top-level `Inbox` folder) with the inbox role, resolves every spelling of a mailbox to one id, refuses `..` and paths outside the root, and an omitted mailbox reaches the inbox without an alias.
- `maildir.layout` selects the fs layout or Maildir++.
- The root of a local store is never deleted or renamed; a Maildir holding unlisted folders is not deleted.
- New error codes `mailbox-not-found` and `message-not-found`; every "no id matched" refusal carries the latter.
- The raw `maildir` id-taking commands refuse the whole set on an unknown id; `maildir message move` prints the new ids; `--subdir tmp` is gone; `maildir flag set` requires `-f`; `maildir create` and `rename` apply the unified guards.
- A role alias may name a mailbox by its unique name, on every backend.
- mbox refuses to delete or rename the spool and directories, names `mbox.inbox` when `INBOX` cannot resolve, hides and refuses side-file names, creates missing Thunderbird parents, keeps `X-Mozilla-Status` in step, codes a busy lock `store-busy`, retries a stale listing or search once, and the raw copy and move print the target ids.
- The local `body` search clause lowercases Unicode text as `subject` does.
- Library fixes patched by path: io-maildir (store-guards) and io-mbox (himalaya-live-test-fixes).

## Scope / non-goals

An exactly full last page still leads to an empty next page on every backend (the paging contract, F13). An undefined `$VAR` in a root is left raw, as pimalaya-config expands it. The path patches go once io-maildir and io-mbox are released.

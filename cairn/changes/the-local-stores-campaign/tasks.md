---
cairn: tasks
change: the-local-stores-campaign
---

# Tasks

- [x] Maildir root named `INBOX` or `.` with the inbox role, one id per spelling, `..` and outside paths refused, omitted mailbox reaching the inbox
- [x] `maildir.layout` (`fs`, `maildir++`) in the config and config.sample.toml
- [x] Root never deleted or renamed; a Maildir holding unlisted folders refused
- [x] `mailbox-not-found` and `message-not-found` codes, on every "no id matched" refusal
- [x] Raw `maildir` commands: whole-set id lookup, move printing new ids, `--subdir` without `tmp`, `flag set` requiring `-f`, create and rename guards
- [x] Role alias by unique mailbox name
- [x] mbox: spool and directory refusals, `INBOX` without a spool, side-file names, Thunderbird parents and `.msf`, `X-Mozilla-Status`, `store-busy`, dotlock hint, stale retry in listings and searches, raw copy and move ids, cache version 2
- [x] `body` search clause lowercasing Unicode
- [x] io-maildir store-guards and io-mbox himalaya-live-test-fixes patched by path
- [x] Spec, CHANGELOG, MIGRATION
- [x] Leftovers of the re-verification (VMD F7, F9, N1 to N3; VMB N1 to N4; code consistency), logged in cairn/log/2026-11-13-the-local-stores-leftovers.md

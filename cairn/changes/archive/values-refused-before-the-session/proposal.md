---
cairn: change
id: values-refused-before-the-session
status: landed
created: 2026-11-18
---

# Values refused before the session

## Why

Finding HIM-24 of the family's cli-family-analysis-2026-11-15 plan, its first and fifth points:

- Values the `imap`, `smtp`, `jmap` and `msgraph` commands refuse themselves were refused after the secret command ran and the session opened, uncoded, exit 1: `imap fetch abc` logged in and selected the mailbox first. The `gmail` and `msgraph` `request` commands refused an unknown method and an off-origin URL the same way (the class of COH-3).
- `imap status` printed `deleted`, `deletedStorage` and `highestModSeq`, which it never requests, always `null`.

## What

- The refusals move into the grammar, coded `usage-invalid`, exit 2, before the configuration is read: the sequence set of `imap fetch|store|copy|move` (a value parser), `jmap submission create --mail-from` without `--rcpt-to` and the reverse (`requires`), `msgraph messages update` naming no change (a required group), the method and the off-origin URL of `gmail request` and `msgraph request` (possible values, a value parser), and `smtp send --notify never` beside another condition (checked before the session, as clap refuses a conflict).
- `imap status` drops the three keys. Requesting them would take a capability check per item (`CONDSTORE` for `HIGHESTMODSEQ`, `IMAP4rev2` or `QUOTA=RES-STORAGE` for `DELETED` and `DELETED-STORAGE`), a server refusing an item it does not announce with `BAD`; nothing in Himalaya reads them, `imap select` printing `highestModSeq` already.
- Not changed: a value read from a file or stdin (`smtp raw` and `sieve raw` batching, `jmap request` and `jmap query` bodies, a Sieve script) is still read inside the command, reading stdin before a session being refused there anyway.
- Breaking: exit 1 becomes 2, `code` `null` becomes `usage-invalid`, and `imap status` loses three keys.

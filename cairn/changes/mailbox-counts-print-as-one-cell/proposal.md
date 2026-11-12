---
cairn: change
id: mailbox-counts-print-as-one-cell
status: landed
created: 2026-11-12
---

# Mailbox counts print as one cell

## Why

every-list-prints-as-envelopes-do dropped the label row of every list, so `mailbox list --counts` printed two bare numbers between the role and the name, and the reader could not tell the total from the unread count. On 2026-11-12 the owner asked for one `new/total` cell instead, printed only with `--counts`, the column skipped otherwise. The cell counts unread mail, the JSON `unread`: "new" means `\Recent` on IMAP and the `new` directory on Maildir, neither of which the unified API carries.

`--counts` did not mean the same on every backend either: Graph filled both counts without it, and pimdir filled `total` alone with it.

## What

- A mailbox's unread and total counts print as one cell, unread first (`3/120`), a count the backend gave none of as `-`: in `mailbox list` with `--counts` only, the column skipped without it, and in `jmap mailbox query|get`, `gmail labels list|get`, `msgraph mail-folders list|get|child-folders` and `pimdir mailbox list`, which always carry them.
- `mailbox list` on Graph leaves both counts `null` without `--counts`, as every other backend does.
- `mailbox list --counts` on pimdir fills `unread` too, counted as `pimdir mailbox list` counts it.

## Scope / non-goals

The `--json` keys do not change. Maildir and m2dir still leave both counts `null`. The mailbox id and name are untouched: Maildir and m2dir still give the absolute path as the id. IMAP still marks only `INBOX` with a role, imap-special-use-aliases waiting on LIST `RETURN (SPECIAL-USE)`.

---
cairn: change
id: envelopes-print-one-line-each
status: landed
created: 2026-11-12
---

# Envelopes print one line each

## Why

Printed as trees (envelopes-hang-as-trees), a mailbox looked messy to the owner on 2026-11-12: two lines per envelope, the branches and the state words in parentheses made a long listing hard to scan. Looking at `jj log`, `mscan`, notmuch and `mu find`, the owner chose the `mscan` index: one line per message, short fixed columns first and the subject last, which a thread reads the same way with its subject indented. A listing also said nothing of where it stood: which account and backend answered, which mailbox, which page, and how to reach the next.

The date defaults change with it: `%F %R%:z` at the sender's offset made two messages of the same hour read differently and widened every line, so a date now prints as `%F %R` in the local timezone.

## What

- `envelope list|search`, `imap thread` and `pimdir thread` print through `Columns` with no header row: the id, four flag columns (`*` unread, `R` answered, `!` flagged, `@` attachment), the date, the senders' names, then the subject. A thread indents a reply's subject two spaces per level; `pimdir thread` adds the mailboxes before the senders.
- `envelope list|search` print a header (`Account:` with the backend, `Mailbox:`, `Query:` for a search) and a footer (`Page:` with the envelopes it holds, `Next page:` with the `--page` reaching it), their values in one column on every page. The output is no longer `output::Paginated`, its JSON unchanged (`envelopes`, `nextPage`), the schema now titled `EnvelopeListOutput`.
- `envelope.list.datetime-fmt` defaults to `%F %R` and `envelope.list.datetime-local-tz` to true.
- `EmailClient::resolution` gives a command the account name and the backend it resolved to.

## Scope / non-goals

The header and footer are himalaya's own, not a pimalaya-cli type yet. The other wide lists stay `Records`. The `--json` output does not change.

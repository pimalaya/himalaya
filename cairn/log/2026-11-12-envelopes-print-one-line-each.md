---
cairn: log
change: envelopes-print-one-line-each
landed: 2026-11-12
---

# Envelopes print one line each

`envelope list|search`, `imap thread` and `pimdir thread` print one line per message the way `mscan` does, replacing the trees of envelopes-hang-as-trees: the id, four flag columns (`*` unread, `R` answered, `!` flagged, `@` attachment), the date, the senders' names and the subject, a thread indenting a reply's subject two spaces per level. `envelope list|search` print `Account:`, `Mailbox:` and a search's `Query:` above the envelopes, `Page:` and `Next page:` under them, the JSON unchanged. `envelope.list.datetime-fmt` defaults to `%F %R` and `datetime-local-tz` to true. pimalaya-cli is pinned at 0fbe5dd, which drops `tree::Trees`. Capability moved: commands (requirement "Text output is plain text" modified).

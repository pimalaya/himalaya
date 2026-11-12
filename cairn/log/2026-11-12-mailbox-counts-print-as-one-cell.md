---
cairn: log
change: mailbox-counts-print-as-one-cell
landed: 2026-11-12
---

# Mailbox counts print as one cell

A mailbox's unread and total counts print as one cell, unread first (`3/120`), a missing count as `-`: in `mailbox list` with `--counts` only, the column skipped without it, and in `jmap mailbox query|get`, `gmail labels list|get`, `msgraph mail-folders list|get|child-folders` and `pimdir mailbox list`, through `listing::counts`. The owner asked on 2026-11-12 for this `new/total` cell, the two bare numbers left by every-list-prints-as-envelopes-do being unreadable without a label row; it counts unread mail, `new` naming something else on IMAP and Maildir. `mailbox list` on Graph leaves both counts `null` without `--counts`, and on pimdir fills `unread` with it. Capability moved: commands (requirement "Text output is plain text" modified), backends (requirement "Mailbox counts are asked for" added).

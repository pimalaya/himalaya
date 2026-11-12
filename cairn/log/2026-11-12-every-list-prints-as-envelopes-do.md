---
cairn: log
change: every-list-prints-as-envelopes-do
landed: 2026-11-12
---

# Every list prints as envelopes do

Every list prints the way `envelope list` does since envelopes-print-one-line-each: one line per item through `Columns` with no header row, `Account:` and what is listed above, `Page:` and `Next page:` under a paged list. The message lists of JMAP, Microsoft Graph and pimdir print the envelope line; every former records list (attachments, Graph folders, pimdir search threads and attachments, Gmail filters, Gmail history one line per message change) prints one line per item; every columns list lost its label row, a yes-or-no cell printing its word. Lists paged by token (Gmail, Graph, pimdir attachments) print `--page-token` and the count they hold; the `Paginated` wrapper left Himalaya, the JSON keeping `nextPage`. `listing::Listing` frames them all, and the account context carries its name. The owner chose on 2026-11-12 to drop the label row everywhere, Graph's 150-character ids included in the envelope line. Capability moved: commands (requirement "Text output is plain text" modified), config and backends (one scenario and one sentence).

---
cairn: change
id: every-list-prints-as-envelopes-do
status: landed
created: 2026-11-12
---

# Every list prints as envelopes do

## Why

envelopes-print-one-line-each gave `envelope list|search` the `mscan` look: one line per envelope with no header row, between `Account:` and `Mailbox:` above and `Page:` and `Next page:` under. On 2026-11-12 the owner asked for the same on every other list. Three shapes were left: the message lists of the other backends and the wide lists still printed as `git log` records, every other list as columns under an uppercase label row, and none said which account answered or where a page stood.

The owner settled four points that day:

- The label row goes everywhere, even where two cells share a shape (two counts, several opaque ids): the column order in `--help` and the spec says which is which.
- Every list run against an account prints `Account:` and what it lists; only a paged list prints `Page:` and `Next page:`, so an unpaged one ends on its last item.
- Every message list prints the envelope line, Microsoft Graph included, though its ids of about 150 characters push the subject far right and fold.
- Every records list goes one line per item, Gmail filters and history included.

## What

- A `listing::Listing` in Himalaya: the header fields, the items through `Columns` with no header row, the footer fields, values aligned two spaces after `Next page:`. `ListingPage` says how a page is reached: by number (`Page: 2, envelopes 26 to 50`, `Next page: --page 3`) or by token (`Page: 25 messages`, `Next page: --page-token <token>`). `envelope list|search` move onto it unchanged.
- The account context carries its name, set by every client builder, for the `Account:` field of the backend namespaces.
- The message lists print the envelope line (`envelope_cells`): `jmap email query|get` (its date `receivedAt`), `msgraph messages list`, `pimdir search` (its mailboxes and what matched before the senders, the snippet left to `--json`), and the threads `imap thread` and `pimdir thread` gain their header. `pimdir tag` and `pimdir queue list` print the same shape with what they carry: the tag's status, mailbox and queue row; the queue's `queued` or `parked`, when, the action and the recipients.
- Every records list goes one line per item: `attachment list|download`, `pimdir attachment list|search`, `msgraph attachments list`, `msgraph mail-folders list|get|child-folders`, `pimdir search --threads`, `gmail settings filters list`, and `gmail history list` one line per message change.
- Every columns list loses its label row and gains the header. A yes-or-no cell prints its word (`default`, `active`, `permanent`, `subscribed`) instead of `yes` or `true`. Sizes print without a blank (`1KiB`), so a line still splits on blanks; the `pimdir tag` statuses print hyphenated (`flags-unknown`) for the same reason.
- The paged lists print their footer: the Gmail lists, `msgraph messages list`, `jmap email|mailbox|submission query`, `pimdir search` and `pimdir attachment list|search`. Their `--json` keeps `nextPage` where it had one, the schemas titled after the output rather than `Paginated`. A pimdir answer's `Incomplete:` line, the queue's cancel hint and Gmail's new history id become footer fields.

## Scope / non-goals

The `--json` output does not change. `account list` and the static `maildir flag list` and `mbox flag list` read no account, so they print their items alone. The header and footer stay Himalaya's own, not a pimalaya-cli type yet; the other Pimalaya CLIs still print columns and records. Records (`Fields`) are untouched.

---
cairn: delta
change: envelopes-hang-as-trees
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Text output is plain text
A text output SHALL be plain text: no border, no colour, nothing sized to or truncated for the terminal, and it SHALL end with a newline, the explicitly requested raw bytes (`message read --raw`, `attachment download --stdout`, `message parse --part` and `--section`) staying byte-exact.

A list whose items hang from a parent (a mailbox, the message they reply to) SHALL print through pimalaya-cli's `Trees`, the way `tree` prints a directory: each message's subject on its `├─ ` or `└─ ` branch, then on the line under it its date as `envelope.list.datetime-fmt` and `datetime-local-tz` configure it, its id followed by its state words in parentheses, comma-separated (`unread` for no `\Seen`, `answered`, `flagged`, and `attachment` only when the backend says the message has one, in that order), then its addresses, two spaces apart, each part only when it holds something. An address SHALL print whole, `Name <email>` or the bare email, several joined with `, `. These lists print as trees:

- `envelope list|search`: one tree whose root is the mailbox listed, its alias resolved, and whose branches are the envelopes in the listing's order; the senders, or `to ` and the recipients with `--recipient`. The size SHALL NOT print.
- `imap thread`: one tree per thread, its first message at the root: a run of members nests each under the one before, the answers hang under the last, and the answers of a message the mailbox lacks stand side by side at its level; the senders.
- `pimdir thread`: each message under the earlier one its `In-Reply-To` names, one whose parent the account lacks being a root of its own; after the state, `in ` and its mailboxes, then the senders.

Any other list whose items carry more than one value of unbounded length (an address, a subject, a path, a long opaque id, criteria) SHALL print through pimalaya-cli's `Records`, the way `git log` prints commits: per item, its id alone on a line, followed by its state words in parentheses, comma-separated, when it has any; then its properties as `Fields` prints them; then its free text after a blank line, each line indented by four spaces; one blank line between items. A field every item carries SHALL print even when empty, its label alone; a field only some items carry SHALL be left out where it is empty. A line a command prints under its records (what a search could not see, a queue note, the `Next page:` of `pimdir attachment list|search`) SHALL print after a blank line. These lists print as records:

- `jmap email query|get`: the email id; the envelope's words from `$seen`, `$answered`, `$flagged` and `hasAttachment`; `From` and `Date` (`receivedAt`); the subject.
- `msgraph messages list`: the message id; `unread`, `flagged` (a follow-up flag `flagged`) and `attachment`; `From` and `Received`; the subject.
- `pimdir search`: the message id; the envelope's words; `From`, `Date`, `Mailbox` and, when a text term matched, `Matched` (`body` or `headers`); the subject, then the snippet after a blank line. `pimdir search --threads`: the thread id, `-` for a message no thread holds; `Messages`, `Last`, `Hits` and `From`; the subject of its first hit.
- `pimdir tag`: the message id; its status; `Mailbox`, `Flags` and, when one was staged, `Row`; the subject.
- `pimdir queue list`, for its queued messages: the row; the envelope's words; `Action` (`save` or `send`), `Queued` and `To`; the subject. Its parked rows SHALL stay columns.
- `attachment list|download`: the part id; `inline` for an inline part; `Filename`, `Type`, `Size` and, once written, `Path`.
- `pimdir attachment list|search`: the file id; `Name`, `Type`, `Size`, `Section`, `Message`, `Mailbox`, `Date` and `From`.
- `msgraph attachments list`: the attachment id; `inline`; `Name`, `Type` and `Size`.
- `msgraph mail-folders list|get|child-folders`: the folder id; `Name`, `Total`, `Unread` and `Children`.
- `gmail settings filters list`: the filter id; `Criteria` and `Action` as `get` prints them.
- `gmail history list`: under its `New history id` field and a blank line, the record id; `Added` and `Deleted`, the message ids joined with `, `, and `Labeled` and `Unlabeled`, one `<message id> (<labels>)` line per message, each only when not empty.

Every other list SHALL print through pimalaya-cli's `Columns`: a header of uppercase labels, then one item per line, the cells two spaces apart and padded to the widest of their column, an empty cell as `-`, a line break inside a cell as a space, and no blank line before it. The free-text column (a name, a snippet, a detail) SHALL read last, and a cell holding several values SHALL join them with `,`, so a line splits on blanks. `jmap thread get` SHALL list one email per line, beside its thread.

A record SHALL print through pimalaya-cli's `Fields`: one `Label: value` line per field, the values aligned past the longest label, a value of several lines continuing under its first. Several records SHALL print a blank line apart. The template a composer prints without `--save` or `--send` SHALL stay the RFC 5322 template an editor round-trips.

#### Scenario: An envelope list
- GIVEN the inbox holding an unseen flagged message `42` from Alice with an attachment, then a seen answered one `41` from Carol
- WHEN `envelope list` prints them
- THEN it prints `inbox`, `├─ ` and the first subject, `│  ` then its date, `42 (unread, flagged, attachment)` and `Alice <alice@example.org>` two spaces apart, then `└─ ` and the second subject, three spaces then its date, `41 (answered)` and Carol's address, with no header, no border and no colour

#### Scenario: A thread
- GIVEN an IMAP `THREAD` answering `(1 (2 4)(3))`
- WHEN `imap thread` prints it
- THEN message 1's subject is the root and its line follows `│  `, 2 and 3 hang from it on `├─ ` and `└─ `, and 4 hangs from 2 on `│  └─ `

#### Scenario: A mailbox list
- GIVEN an account holding two mailboxes
- WHEN `mailbox list` prints them
- THEN it prints a header of uppercase labels ending with `NAME`, then one aligned line per mailbox

## REMOVED Requirements

### Requirement: Envelope dates MAY render relative to today
Removed: dates print as `envelope.list.datetime-fmt` and `datetime-local-tz` configure them, and the key is refused as unknown.

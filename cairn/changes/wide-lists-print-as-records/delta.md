---
cairn: delta
change: wide-lists-print-as-records
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Text output is plain text
(commands) A text output SHALL be plain text: no border, no colour, nothing sized to or truncated for the terminal, and it SHALL end with a newline, the explicitly requested raw bytes (`message read --raw`, `attachment download --stdout`, `message parse --part` and `--section`) staying byte-exact.

A list whose items carry more than one value of unbounded length (an address, a subject, a path, a long opaque id, criteria) SHALL print through pimalaya-cli's `Records`, the way `git log` prints commits: per item, its id alone on a line, followed by its state words in parentheses, comma-separated, when it has any; then its properties as `Fields` prints them; then its free text after a blank line, each line indented by four spaces; one blank line between items. A field every item carries SHALL print even when empty, its label alone; a field only some items carry SHALL be left out where it is empty. An address SHALL print whole, `Name <email>` or the bare email, several joined with `, `. A line a command prints under its records (what a search could not see, a queue note, the `Next page:` of `pimdir attachment list|search`) SHALL print after a blank line. These lists print as records:

- `envelope list|search`: the envelope id; `unread` (no `\Seen`), `answered`, `flagged` and `attachment` (only when the backend says the message has one), in that order; `From`, or `To` with `--recipient`, `Date` as `envelope.list.datetime-*` configure it, and `Size`; the subject.
- `jmap email query|get`: the email id; the same words from `$seen`, `$answered`, `$flagged` and `hasAttachment`; `From` and `Date` (`receivedAt`); the subject.
- `msgraph messages list`: the message id; `unread`, `flagged` (a follow-up flag `flagged`) and `attachment`; `From` and `Received`; the subject.
- `pimdir search` and `pimdir thread`: the message id; the envelope's words; `From`, `Date`, `Mailbox` and, when a text term matched, `Matched` (`body` or `headers`); the subject, then the snippet after a blank line. `pimdir search --threads`: the thread id, `-` for a message no thread holds; `Messages`, `Last`, `Hits` and `From`; the subject of its first hit.
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
- GIVEN a mailbox holding an unseen flagged message `42` from Alice with an attachment, then a seen answered one `41` from Carol
- WHEN `envelope list` prints them
- THEN it prints `42 (unread, flagged, attachment)`, `From:  Alice <alice@example.org>`, its `Date:` and `Size:` lines, a blank line and its subject indented by four spaces, then a blank line and `41 (answered)` with its fields and subject, with no header, no border and no colour

#### Scenario: A mailbox list
- GIVEN an account holding two mailboxes
- WHEN `mailbox list` prints them
- THEN it prints a header of uppercase labels ending with `NAME`, then one aligned line per mailbox

### Requirement: No colour is configured
(config) The configuration SHALL hold no colour, no table shape and no glyph: `*.list.table.*-color`, `table.preset` and `table.arrangement` are unknown keys, refused where their table denies unknown keys (`mailbox`) and ignored at the top level and the account level, which tolerate unknown keys for himalaya-tui. The `envelope.list.table` block SHALL be ignored whatever it holds, its v2 glyph and colour keys included, while `envelope.list` keeps refusing an unknown key of its own.

#### Scenario: A v2 glyph key
- GIVEN an account setting `envelope.list.table.unseen-char = "N"`
- WHEN `envelope list` runs
- THEN the configuration loads and an unread message reads `unread` among its state words

#### Scenario: A typo under envelope.list
- GIVEN an account setting `envelope.list.datetime-fnt`
- WHEN any command reads the configuration
- THEN it is refused naming `datetime-fnt`

### Requirement: Envelope dates MAY render relative to today
(config) `envelope.list.datetime-relative`, settable globally and per account, SHALL render the `Date` field of `envelope list` and `envelope search` relative to today in the local timezone: the time of day (`%R`) for today, `yesterday` for the day before, the weekday (`%A`) for the five days before that, and `datetime-fmt` for anything older or dated in the future. It SHALL default to `false`, and SHALL NOT change the `--json` output, which keeps the exact date.

### Requirement: The envelope carries its threading pointers
(backends) The last paragraph becomes: The field SHALL NOT be printed by the `envelope list` text output, where raw msg-ids would be noise; it rides the JSON output. The rest is unchanged.

### Requirement: The pimdir subcommand reads and retracts the queue
(backends) The second paragraph ends: the row id is the row's own `id`, the title of its record. The rest is unchanged.

## REMOVED Requirements

None.

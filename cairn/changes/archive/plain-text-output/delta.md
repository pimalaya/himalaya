---
cairn: delta
change: plain-text-output
---

# Delta

## ADDED Requirements

### Requirement: Text output is plain text
(commands) A text output SHALL be plain text: no border, no colour, nothing sized to or truncated for the terminal, and it SHALL end with a newline, the explicitly requested raw bytes (`message read --raw`, `attachment download --stdout`, `message parse --part` and `--section`) staying byte-exact.

A list SHALL print through pimalaya-cli's `Columns`: a header of uppercase labels, then one item per line, the cells two spaces apart and padded to the widest of their column, an empty cell as `-`, a line break inside a cell as a space, and no blank line before it. The free-text column (a subject, a name, a snippet) SHALL read last, and a cell holding several values SHALL join them with `,`, so a line splits on blanks. `jmap thread get` SHALL list one email per line, beside its thread.

A record SHALL print through pimalaya-cli's `Fields`: one `Label: value` line per field, the values aligned past the longest label, a value of several lines continuing under its first. Several records SHALL print a blank line apart. The template a composer prints without `--save` or `--send` SHALL stay the RFC 5322 template an editor round-trips.

#### Scenario: An envelope list
- GIVEN a mailbox holding an unseen flagged message from Alice and a seen one from Bob Martin
- WHEN `envelope list` prints them
- THEN it prints the header `ID`, `FLAGS`, `FROM`, `DATE`, `SIZE`, `SUBJECT`, then one aligned line per envelope, the FLAGS of the unseen one `*!` and of the seen one `-`, with no border and no colour

### Requirement: Help is plain text
(commands) `--help` and `-h` SHALL print without colour, each paragraph on one line, never wrapped to the terminal width.

### Requirement: A query error is drawn without colour
(commands) The report `envelope search` and the pimdir search commands draw over a query they refuse SHALL carry no colour escape, on a terminal as in the `--json` error string.

### Requirement: No colour is configured
(config) The configuration SHALL hold no colour and no table shape: `*.list.table.*-color`, `table.preset` and `table.arrangement` are unknown keys, refused where their table denies unknown keys (`envelope.list.table`, `mailbox`) and ignored at the top level and the account level, which tolerate unknown keys for himalaya-tui. `envelope.list.table.unseen-char`, `replied-char`, `flagged-char` and `attachment-char` SHALL keep naming the FLAGS and ATT glyphs, a FLAGS cell holding only the glyphs that apply.

## MODIFIED Requirements

### Requirement: The envelope carries its threading pointers
(backends) The shared `Envelope` SHALL carry `message_id` and `in_reply_to`, the RFC 5322 §3.6.4 identity of a message and of the message(s) it replies to, so a client can pair a reply with its parent from a listing rather than by reading bodies.

`in_reply_to` SHALL be a list, the grammar being `1*msg-id`, and every id in it SHALL be normalised exactly as `message_id` is (angle brackets and surrounding whitespace stripped), so the two compare byte-for-byte whatever backend surfaced them.

Each backend SHALL source the field from the response its listing already makes, and SHALL leave it empty rather than issue a request of its own: IMAP from the `ENVELOPE` (RFC 3501 §7.4.2, 9th element), JMAP from the `inReplyTo` property of `Email/get`, Gmail from the metadata headers, Maildir, m2dir and mbox from the parsed message, and pimdir from the stored summary. Graph leaves it empty, `In-Reply-To` living in `internetMessageHeaders`, which a listing selection does not return.

The field SHALL NOT take a column in the `envelope list` text output, where a column of raw msg-ids would be noise; it rides the JSON output.

### Requirement: Mailbox role
(backends) A shared mailbox SHALL carry an optional role (inbox, all, archive, drafts, flagged, important, junk, sent, subscribed, trash, or a verbatim unknown one), shown by `mailbox list` in its text and JSON output. JMAP reads it from the mailbox `role`, Gmail from its fixed system-label ids, Microsoft Graph from its well-known folder names resolved to folder ids in one `$batch` request, pimdir from the role its sync engine recorded from the server (`collections.role`), and IMAP marks only `INBOX`, as mbox marks its spool when one is configured. Maildir and m2dir have no native role. On JMAP, Gmail, Graph and pimdir a role name addresses the mailbox carrying it. A `mailbox.alias.<role>` entry overrides the native role of the mailbox it names, and `message delete` takes its trash from that alias before the backend's trash role.

## REMOVED Requirements

None: `-w/--max-width` and the colour keys were never spec requirements.

---
cairn: spec
capability: commands
status: current
---

# Commands

The command tree splits into three groups. The shared API (mailbox, envelope, flag, message, attachment) is the cross-protocol least-common-denominator surface, behaving the same whatever backend serves the active account. The protocol-specific APIs (imap, jmap, gmail, msgraph, maildir, m2dir, mbox, pimdir, smtp, sieve) each expose the full surface of one backend, including operations the shared API cannot model. The meta commands (account, completion, manual, json-schema) cover account configuration, shell completions, man pages and JSON Schemas.

### Requirement: Shared commands over a selected backend
The shared commands SHALL run over an `EmailClient` that owns one backend-client variant per compiled-in backend. It selects the first configured storage backend the global `--backend` flag allows, preferring local backends over network ones, plus an optional SMTP transport for storage backends that cannot send (IMAP, Maildir, m2dir, mbox). Each shared method matches the active backend and calls its per-protocol adapter.

### Requirement: Protocol commands ignore backend selection
Each protocol command SHALL build its own `<Proto>Client` via a `build_<proto>_client` helper and run against that backend directly, ignoring `--backend`. The imap command mirrors IMAP's flat command list; gmail and msgraph track their REST resource domains; the filesystem backends expose only operations that map to their on-disk layout, leaving MIME rendering to the shared commands.

### Requirement: ManageSieve commands

When built with the `sieve` feature, the protocol-specific API SHALL expose `sieve capability`, `list`, `get`, `put`, `check`, `activate`, `deactivate`, `rename`, `delete`, and `raw`. Data commands SHALL use dedicated serializable output types; mutation commands SHALL use confirmation messages. `put` and `check` SHALL accept script bytes from a file, inline arguments, or stdin, and SHALL report the warning text a server attaches to an accepted script.

The protocol SHALL live in io-managesieve, so this repository holds a `SieveClient` wrapping `ManagesieveClientStd`, one file per subcommand, and its own serializable output types, exactly as the other protocol modules do.

### Requirement: Raw passthrough is byte-verbatim
The `imap raw` and `smtp raw` commands SHALL forward the command bytes to the server verbatim, resolving the argument through the shared `RawCommandArg` (positional or stdin). It decodes literal `\r` / `\n` escapes into real CRLF so a shell-typed command survives intact. `imap raw` sends a batch of caller-tagged commands: it appends a trailing CRLF when missing and delegates tagging, framing and out-of-order completion tracking to io-imap. `smtp raw` stays a single command line: it strips the trailing CRLF (io-smtp appends its own) and rejects a multi-line batch, since the SMTP exchange reads exactly one reply. `sieve raw` sends one command line and reads the complete ManageSieve response; literal-bearing Sieve commands use their typed commands so the response reader remains synchronized.

### Requirement: Account threaded as a sibling argument
The active account context SHALL be threaded as a sibling argument through every `execute` chain, never reached through the client. Subcommands receive `account` and `client` side by side.

### Requirement: Output discipline
Data and errors SHALL go to stdout through the printer; `--json` switches every command to JSON. stderr carries logs only. Each command's doc comment is its `--help` text, so `himalaya <command> --help` is the canonical per-command usage reference.

### Requirement: A JSON error carries a stable code
A failure a caller is expected to act on SHALL carry a stable code, printed under `--json` as a `code` string beside `error`, `sources` and `backtrace`. The code SHALL be found anywhere in the error chain, so added context does not hide it. A failure with no code SHALL print no `code` field. The plain output SHALL stay unchanged. Codes are kebab-case and never renamed; `error` stays free wording.

| Code | Raised when |
|---|---|
| `body-pending` | a listed message's body is not local yet (pimdir); a sync brings it |
| `message-too-complex` | a message is nested deeper than 8 levels, or holds more than 200 parts or 500 header lines; it is refused whole |

#### Scenario: A body not downloaded yet
- GIVEN a pimdir item whose body is not local
- WHEN `himalaya --json message read` reads it
- THEN the command exits 1 and prints `{"code":"body-pending","error":"…","sources":[],"backtrace":null}`

### Requirement: Plain output is printable
A single-line field taken from a message or a server (a subject, a name, a filename, a mailbox name, an id) SHALL have its control characters (C0, DEL and C1) and bidi controls (U+202A to U+202E, U+2066 to U+2069) replaced with U+FFFD before it is printed as plain output, so it can neither drive the terminal nor reorder what is displayed. Message bodies keep their tabs and newlines; on a terminal they SHALL be refused as binary when they hold any other control character, DEL and C1 included. The `--json` output SHALL keep the strings unchanged.

### Requirement: Generation commands print to the standard output
The `completion`, `manual` and `json-schema` commands SHALL share one shape: a positional list selecting what to generate, defaulting to everything, and an optional `--dir` deciding where it lands. Without a directory they SHALL print the single selected item to the standard output, so a packaging helper can capture it and a shell can redirect it to a file, and SHALL fail when several items are selected rather than concatenating pieces valid for nothing. With a directory they SHALL write one file per selected item in it, creating the directory when missing, and report where each landed.

### Requirement: Data commands serialize their data
A command returning data SHALL hand the printer a dedicated output type implementing both `Display` and `Serialize`, and register its JSON Schema under the command's invocation key. `Message` is reserved for confirmations, since it serializes as a single `message` string and leaves `--json` unparseable. Where a sibling `list` already serializes a backend resource, the `get` output SHALL emit that resource verbatim through a transparent newtype, so one item read with `get` has the shape of one row of `list`. Where the wire type is unsuitable (a recursive MIME tree, a type carrying no schema), the output type SHALL name its fields instead. `gmail threads get` is the exception for the MIME tree: `users.threads.get` has no raw format, so each message's `payload` SHALL be serialized when the requested format carries one.

### Requirement: Serialized collections are always present
An output field holding a collection SHALL be serialized even when empty, because the schema marks it required regardless and a skipped field would contradict the published schema.

### Requirement: Gmail header selection applies to every format
`gmail messages get --header` and `gmail threads get --header` SHALL narrow the rendered headers whatever the requested format. Gmail honours its `metadataHeaders` parameter under the metadata format alone and returns every header otherwise, so the narrowing SHALL also be applied to the response. Matching is case-insensitive, order and repeats are preserved, and passing no `--header` renders every header. Headers are read from the top-level payload part, where Gmail puts the RFC 5322 headers.

### Requirement: Raw message formats write bytes
A Gmail `get` command asked for the raw format SHALL decode the fetched message and write its RFC 5322 bytes through the shared byte writer rather than rendering a summary. This covers `messages get` and `drafts get`.

### Requirement: One module per subcommand
A protocol command resource SHALL live in a sibling module file carrying its `pub mod` declarations and its `Command` enum, next to a folder holding one file per subcommand. Types live with the subcommand that owns them; a type serving several subcommands lives in the file of the one that owns it, or in its own module when it belongs to none.

### Requirement: Command types carry a domain and a target
A protocol command type SHALL be named `<Domain><Target><Verb>Command`, so a bare verb is never a type name. The domain prefix stays wherever the bare name would collide across the backends a CLI spans, and is otherwise omitted for the tables and value enums under the cli subtree.

### Requirement: Composers default the From header
`messages compose`, `messages reply` and `messages forward` SHALL fill the `From` header from the resolved account when `--from` is not passed: `email` as the address, `display-name` as the name it carries. An explicit `--from` SHALL win whole, the configured name never being grafted onto an address the user spelled out. With neither, the header SHALL be omitted rather than guessed.

The name SHALL be handed to the MIME builder apart from the address, so that a name carrying a comma, a quote or a non-ASCII character is encoded by the builder rather than by a quoting rule of Himalaya's own.

The same composers SHALL end the body with the account's `signature`, introduced by its `signature-delim`, when neither `--signature` nor `--signature-file` is passed. `--signature` SHALL win, and `--signature-file` SHALL win too, the configured signature standing down rather than shadowing the file the flag names.

With `--json` and neither `--save` nor `--send`, the same composers SHALL print the decoded `from`, `to`, `cc`, `bcc`, `subject` and `body` instead of the raw bytes, the body without any signature, so an editor can lay the message out and hand it back through the flags, sending appending the signature once. `--posting-style none` SHALL send the written body alone, the source left unquoted, for a writer who already laid the quote out.

### Requirement: Raw message input is shared
A command taking a raw RFC 5322 message SHALL resolve it through the shared `MessageArg`: a file path, an inline value after `--`, or piped stdin. The resolved message is normalised to CRLF and rejected when empty, so no backend receives a zero-length message.

### Requirement: Gmail draft writes return identities
`gmail drafts create` and `gmail drafts update` SHALL serialize the returned draft `id`, and nullable `message-id` and `thread-id`, under `--json`, with schemas registered for both commands. Non-JSON output SHALL retain the existing success sentence. A response without a message SHALL still succeed, with `message-id` and `thread-id` null, since the draft was already written.

### Requirement: Gmail history retains added-message details
`gmail history list --json` SHALL retain each added message's `id`, nullable `thread-id`, and supplied `label-ids` in a `messages-added-details` array, preserving order and duplicate entries. Missing labels SHALL produce an empty array. The existing `messages-added` id array, other change arrays, pagination, and text counts SHALL remain unchanged. The registered JSON Schema SHALL describe the details array, which SHALL remain present when empty.

### Requirement: IMAP fetch can return whole messages
`imap fetch --body` SHALL add `BODY.PEEK[]` (RFC 3501 §6.4.5) to the requested data items, so a caller downloads every message of a sequence set over one session instead of one `message read` per message. The fetch SHALL NOT set `\Seen`. The JSON output SHALL carry the message octets byte-exact as the standard Base64 of the `body` field, which the JSON Schema SHALL declare with `contentEncoding: base64` and `contentMediaType: message/rfc822`; the plain rendering SHALL print the size only. `--body` SHALL NOT imply `--envelope`.

### Requirement: Saving a sent message follows the send
A command both saving and sending (`--save` with `--send`, `message send --save`, `message add --send`) SHALL send first and save afterwards, so a failed send leaves no copy. A save failing after a successful send SHALL fail the command with an error stating that the message was sent.

### Requirement: A read message is a designed view
`message read --json` SHALL print `{headers, text, html, parts}`, built from the raw message so it is the same on every backend, rather than the parser's own model. `headers` SHALL carry `from`, `to`, `cc`, `bcc`, `replyTo` and `sender` as lists of `{name, email}` (groups flattened, an entry without a valid email dropped, `name` decoded or `null`), `subject` decoded, `date` as RFC 3339 with its offset or `null`, `messageId`, `inReplyTo` and `references` (a list) without angle brackets, and `listId`, `listUnsubscribe`, `precedence` and `autoSubmitted` as found, decoded. `text` SHALL be every `text/plain` body part decoded to UTF-8 by its charset, lossily, joined by a blank line, and `html` the first `text/html` body part, each `null` when absent. The text output and `--raw` SHALL stay unchanged.

### Requirement: Malformed mail reads as its sender meant
The view SHALL read a header from its raw bytes: raw 8-bit bytes SHALL be decoded as UTF-8 when they are valid UTF-8 (RFC 6532) and as Windows-1252 otherwise, and adjacent RFC 2047 encoded words of one charset, separated by linear whitespace alone, SHALL be joined as bytes before their charset is decoded, the whitespace between adjacent encoded words being dropped (RFC 2047 §6.2). This applies to the subject, the display names, the list headers and the file names; an RFC 2231 file name SHALL win over a plain one. A text part labelled `us-ascii` or `ascii`, or naming no charset, whose bytes are valid UTF-8 SHALL be read as UTF-8, in `text` and `html` alike. A `Content-Type` whose type or subtype is missing, empty or not an RFC 2045 token SHALL be taken as `text/plain; charset=us-ascii` (RFC 2045 §5.2), its part reporting that `mime` and `charset`.

### Requirement: The read view lists unreadable address entries
`headers.invalid` SHALL be an object `{to, cc, bcc, replyTo, from, sender}`, each a list of the entries of that header that are not readable as an address and so are absent from its list, as decoded text (encoded words decoded, trimmed), in header order, empty when none. A group name SHALL NOT be an entry, nor an empty group. `message read --json` and `message parse --json` SHALL print the same.

### Requirement: Every part has an id and a role
`parts` SHALL list every leaf part, depth first, as `{id, role, mime, filename, size, contentId, charset, method}`, the `id` being the one `attachment list` and `attachment download` take: the 1-based position of the part in the whole part list. A `multipart/*` SHALL be no part; `message/rfc822` or `Content-Disposition: attachment` SHALL be an `attachment`; a `text/plain` or `text/html` without a filename SHALL be a `body`; an `image/*` marked inline or carrying a `Content-ID` SHALL be `inline`; anything else SHALL be an `attachment`. `filename` SHALL be RFC 2231 and RFC 2047 decoded, `null` when the part names none; `size` SHALL count the part's bytes once its transfer encoding is undone, in its own charset; `contentId` SHALL carry no angle brackets; `method` SHALL be a `text/calendar` part's `method` parameter, uppercased. `attachment list --json` SHALL report the same ids, sizes and `contentId`.

### Requirement: A message too complex is refused whole
A message nested deeper than 8 levels, or holding more than 200 parts or 500 header lines (folded lines and the headers of every part and attached message included), SHALL be refused with the JSON error code `message-too-complex` rather than read in part, by `message read` (except `--raw`) and the `attachment` commands.

### Requirement: A part can be read without touching the disk
`attachment download <MESSAGE-ID> <PART-ID> --stdout` SHALL take exactly one part id, any leaf part `message read` lists, and write that part's decoded bytes to stdout with nothing else, touching no file; `--dir` SHALL be refused beside it. Under `--json` it SHALL print `{id, mime, filename, size, data}`, `data` in base64. Without `--stdout`, the attachment parts SHALL be written to the download directory as the same decoded bytes.

### Requirement: A message can be read without an account
`message parse [EML]` SHALL read a raw message from a path, or from stdin with `-` or when omitted, and SHALL print what `message read` prints for the same bytes. It SHALL resolve no account and read no configuration. A source holding nothing but whitespace SHALL be refused, naming where it was read from; the bounds and the `message-too-complex` code SHALL apply as on `message read`.

### Requirement: A parsed message hands one part over
`message parse --part <PART-ID>` SHALL write that part's bytes, transfer encoding undone, to stdout, or under `--json` `{id, mime, filename, size, data}` with `data` in base64; the ids SHALL be those of the view. An unknown id or a container's SHALL be refused.

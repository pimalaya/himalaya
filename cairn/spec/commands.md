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

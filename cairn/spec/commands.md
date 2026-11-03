---
cairn: spec
capability: commands
status: current
---

# Commands

The command tree splits into three groups. The shared API (mailbox, envelope, flag, message, attachment, vacation) is the cross-protocol surface: the union of what the backends serve, held coherent by one meaning per verb, a backend that cannot give a verb's result declaring it unserved rather than giving another. The protocol-specific APIs (imap, jmap, gmail, msgraph, maildir, m2dir, mbox, pimdir, smtp, sieve) each expose the full surface of one backend, including operations the shared API cannot model. The meta commands (account, repl, mcp, completion, manual, json-schema) cover account configuration, a session running many commands (cairn/spec/repl.md), an MCP server (cairn/spec/mcp.md), shell completions, man pages and JSON Schemas.

### Requirement: Shared commands over a resolved transport
The shared commands SHALL run over an `EmailClient` owning one backend-client variant per compiled-in backend, each verb resolved to a configured transport. The verbs exchanging ids resolve to the storage backend: the first configured one the global `--backend` flag allows, local before network (Maildir, m2dir, mbox, pimdir, JMAP, Gmail, Graph, IMAP). Sending resolves to the storage backend when it sends (JMAP, Gmail, Graph, pimdir), else to the SMTP transport, which SHALL connect for each send and close after it. `--backend` SHALL filter the resolution. Each shared method matches the resolved backend and calls its per-protocol adapter.

### Requirement: Capability registry
Each backend SHALL declare, in one table per backend (src/capability.rs), the shared capabilities it serves, named in pimdir STORAGE Annex B's grammar without the domain word (`message.move`, `flags.answered`, `submit`, and `envelope.search.<option>` for each clause and sort key of the query), at a level: `full`; `partial`, with a detail saying what differs; `server`, with a detail saying what the server must advertise; or `none`, with a detail saying why and what serves it instead. A backend SHALL declare a capability served only when its result is the one every other declarer gives, and declare `none` otherwise. The tables SHALL be compiled into every build, compiled-out backends included.

### Requirement: An unserved verb is refused before connecting
A shared command SHALL be checked against the capabilities its arguments reach (each flag it names, each clause and sort key of its query, `--save`, `--send`, `--seen`) before any connection, on the resolution of the account's blocks or of the kept session client, so a refusal resolves no credential, opens no TLS and drops no kept client. A capability no configured transport answers, or one its transport declares `none`, SHALL fail with the code `capability-missing` and a message naming the account, the capability, the backend or the `--backend` filter, and the detail; a query option SHALL fail with `search-unsupported` instead. A copy `message.send.save-copy` configures SHALL be checked once the account is read, still before the send. An account configuring no transport at all SHALL keep the error naming its blocks.

#### Scenario: Answered on Gmail
- WHEN `himalaya flag add 42 --flag answered` runs on a Gmail account
- THEN it fails with `capability-missing` naming `flags.answered`, before the token command runs, where it used to report the message flagged and change nothing

### Requirement: A server-decided capability is refused once connected
A capability declared `server` SHALL be checked by the adapter once connected, before any write. A JMAP send whose session lacks `urn:ietf:params:jmap:submission` (RFC 8621 section 7) SHALL fail with `capability-missing` before the upload, leaving no draft. An IMAP removal without UIDPLUS SHALL flag the messages `\Deleted` and say so. A JMAP `vacation` whose session lacks `urn:ietf:params:jmap:vacationresponse` SHALL fail with `capability-missing`, and an IMAP keyword the mailbox would not keep SHALL be refused as "An IMAP keyword is kept or refused" says.

### Requirement: Verbs exchanging ids share one transport
The shared verbs that take or return message, envelope or mailbox ids (mailbox, envelope, message, flag, attachment) SHALL resolve to the same storage transport for a given account and `--backend`, so an id one of them returns is valid in the others. A verb exchanging no such id, sending or the auto-reply, SHALL resolve on its own.

### Requirement: An account feature resolves on its own
A capability of the account rather than of a message, the auto-reply, SHALL resolve to the first backend of the storage order the account configures and `--backend` allows that serves it, else to the storage backend, whose refusal names what serves it instead.

#### Scenario: A Maildir synced from JMAP
- GIVEN an account configuring a Maildir and a JMAP block
- WHEN `vacation get` runs
- THEN it reads JMAP's auto-reply, where `envelope list` reads the Maildir

### Requirement: The shared command tree is the same in every build
The shared command tree SHALL be compiled in every build carrying a mail transport (any storage backend, or SMTP alone), whichever backends it compiles, dispatch alone being gated; a verb no compiled backend serves parses and fails with `capability-missing`. The shared commands' `json-schema` output and the error code vocabulary SHALL be identical across such builds. Each shared leaf's `--help` SHALL end with a `Backends:` footer generated from the tables, listing per capability it reaches the backends at each level and the details. A protocol namespace exists with its feature.

### Requirement: Resolution changes no served verb
Resolving through the registry SHALL leave the output of every verb a backend serves unchanged. Only a verb a backend declares `none` changes: from a silent no-op (`flag add answered` and `draft` on Gmail and Graph), an uncoded error (`message add` on Graph) or a failure after the send (`message send --save` on Graph) to `capability-missing` up front.

### Requirement: Protocol commands ignore backend selection
Each protocol command SHALL build its own `<Proto>Client` via a `build_<proto>_client` helper and run against that backend directly, ignoring `--backend`. The imap command mirrors IMAP's flat command list; gmail and msgraph track their REST resource domains; the filesystem backends expose only operations that map to their on-disk layout, leaving MIME rendering to the shared commands.

### Requirement: ManageSieve commands

When built with the `sieve` feature, the protocol-specific API SHALL expose `sieve capability`, `list`, `get`, `put`, `check`, `activate`, `deactivate`, `rename`, `delete`, and `raw`. Every command SHALL print a dedicated serializable output type: `activate`, `rename` and `delete` the receipt `{id}` naming the script, `deactivate` `{active: null}`, `check` and `put` `{name, warnings}`, `warnings` being the text a server attaches to an accepted script, `null` when it sends none, and `name` `null` for a check. `put` and `check` SHALL accept script bytes from a file, inline arguments, or stdin.

The protocol SHALL live in io-managesieve, so this repository holds a `SieveClient` wrapping `ManagesieveClientStd`, one file per subcommand, and its own serializable output types, exactly as the other protocol modules do.

### Requirement: Raw passthrough is byte-verbatim
The `imap raw` and `smtp raw` commands SHALL forward the command bytes to the server verbatim, resolving the argument through the shared `RawCommandArg` (positional or stdin read whole, refused when not valid UTF-8). It decodes literal `\r` / `\n` escapes into real CRLF so a shell-typed command survives intact. `imap raw` sends a batch of caller-tagged commands: it appends a trailing CRLF when missing and delegates tagging, framing and out-of-order completion tracking to io-imap. `smtp raw` stays a single command line: it strips the trailing CRLF (io-smtp appends its own) and rejects a multi-line batch, since the SMTP exchange reads exactly one reply. `sieve raw` sends one command line and reads the complete ManageSieve response; literal-bearing Sieve commands use their typed commands so the response reader remains synchronized.

The reply SHALL be printed split into what himalaya understands of it. `imap raw` SHALL print each untagged response as an `untagged` event `{kind, number, text}` (`* 3 EXISTS` being kind `EXISTS`, number 3), each continuation request as a `continuation` event `{text}`, and a response it cannot split as an `unparsed` event `{text}`, a literal kept in the response announcing it; then the result `{completions: [{tag, status, code, text}]}`. `sieve raw` SHALL print each data line as a `data` event `{text}`, then the completion `{status, code, text}`. `smtp raw` SHALL print `{code, text}`, the reply's lines joined, `code` `null` when the reply opens with none. A refused completion (an IMAP `NO` or `BAD`, a ManageSieve `NO` or `BYE`, an SMTP 4xx or 5xx) SHALL exit 2, its result printed. `imap raw` reads the whole reply before printing, so its events come before the result but not as they arrive, and io-imap decodes the reply lossily, so a non-UTF-8 byte comes back as U+FFFD.

### Requirement: Account threaded as a sibling argument
The active account context SHALL be threaded as a sibling argument through every `execute` chain, never reached through the client. Subcommands receive `account` and `client` side by side.

### Requirement: account check covers every declared backend
`account check` SHALL check every backend the account declares and `--backend` allows, and print the report whole, one row `{backend, ok, code, error}` per backend, `code` and `error` `null` when the backend answered. When any fails, it SHALL exit 2 after the report, so a script reads the outcome from the exit code and each backend's code from its row. A pimdir block SHALL be checked by opening the store as a reader, listing its mailboxes, then opening a producer and probing it (`PimdirProducer::probe`), which takes and releases the staging lock without waiting and the database's write lock, writing nothing into the store, so an account declaring only a pimdir block is checked rather than refused as declaring no backend, and a store a write cannot reach fails the check rather than the first write: a collector holding the staging lock fails it at once with `store-busy`, and the check leaves nothing in objects/.

### Requirement: Output discipline
Data and errors SHALL go to stdout through the printer; `--json` switches every command to JSON. stderr carries logs only. Each command's doc comment is its `--help` text, so `himalaya <command> --help` is the canonical per-command usage reference.

### Requirement: A JSON error carries a stable code
A failure a caller is expected to act on SHALL carry a stable code, printed under `--json` as a `code` string beside `error`, `sources` and `backtrace`. The code SHALL be found anywhere in the error chain, so added context does not hide it. A failure with no code SHALL print `code: null`. The plain output SHALL stay unchanged. Codes are kebab-case and never renamed; `error` stays free wording.

### Requirement: The JSON contract is described and versioned
`json-schema` SHALL describe the failure object under the key `himalaya-error`, its `code` typed by the code vocabulary, and every command printing to stdout under its command key, checked by a test walking the command tree: only a bare `himalaya` (the help, or the wizard `configure` describes), `repl` and `mcp` are left out. A command printing one of two shapes SHALL be described by both, and a command printing events SHALL describe them under `<key>.events`. Schemas SHALL describe a value as printed, so a key always printed is required. Every generated schema SHALL carry an `$id` naming the contract version, the major version of Himalaya. Every registered payload SHALL be a JSON object, checked by a test. Piped `--json` output SHALL print one value per line.


| Code | Raised when |
|---|---|
| `body-pending` | a listed message's body is not local yet (pimdir); a sync brings it |
| `message-too-complex` | a message is nested deeper than 8 levels, or holds more than 200 parts or 500 header lines; it is refused whole |
| `stdin-taken` | a command inside a `repl` session would read its input from stdin, which carries the session's commands; pass it as an argument or a file path |
| `search-invalid` | an `envelope search` query does not parse; nothing is searched |
| `search-unsupported` | an `envelope search` clause the account's backend cannot search, quoted as written; nothing is searched, no clause dropped |
| `capability-missing` | the account's backend does not serve the verb, or the option of it asked for, as its capability table declares or its server advertises; nothing is done, the message naming the capability and what serves it instead |
| `store-version` | a pimdir store is of another draft of the format; upgrade Himalaya, or reinstall the store as neverest's MIGRATION.md says |
| `store-uncreated` | a pimdir store has no schema yet; its sync engine creates it |
| `store-foreign` | the database at a pimdir root is not a pimdir store; another program wrote it |
| `store-corrupt` | a pimdir store's version records disagree, or its schema is not its draft's |
| `store-not-current` | a pimdir store holds no schema yet and the verb creates none; its sync engine creates it |
| `store-hash-algo` | a pimdir store names its bodies with a hash Himalaya does not compute |
| `sync-version` | a pimdir store's sync state was written by another version of its sync engine |
| `store-busy` | another writer holds a pimdir store's write lock; retry |
| `store-owned` | a sync owns a pimdir store; retry once it ends |
| `store-staging` | a producer is staging a body in a pimdir store; retry |
| `store-in-flight` | a pimdir store verb is between two of its chunks; retry |
| `store-exposed` | a pimdir store is a link or another user can read or write it; restrict it |
| `item-duplicate` | a pimdir mailbox holds the message already, or has its creation queued; nothing is staged |
| `flag-invalid` | a flag a pimdir store cannot hold: empty, or holding whitespace, a control or a non-ASCII character; nothing is staged |
| `action-malformed` | a pimdir action's payload breaks its schema; nothing is staged |
| `conflict-unread` | a pimdir conflict holds a body nobody has read; fetch it first |
| `unsupported` | a source of the pimdir store does not support the write; nothing is staged |
| `no-performer` | no source of the pimdir store can perform the intent |
| `ambiguous-performer` | several sources of the pimdir store can perform the intent and none is chosen |
| `index-missing` | a pimdir store has no search index yet; its sync engine builds it |
| `index-outdated` | a pimdir search index was built by another version; its sync engine rebuilds it |
| `index-version` | a pimdir search index is of another draft or shape; its sync engine rebuilds one of an earlier draft |
| `index-zone-unknown` | the zone a pimdir search index reads floating times in is no IANA zone |
| `index-fts5-missing` | the SQLite Himalaya links lacks what a pimdir search index needs (FTS5, 3.43 or later) |
| `query-refused` | a pimdir search query breaks a SEARCH §8.4 rule; nothing is searched |
| `query-unanswerable` | a pimdir search index could not answer an accepted query, an implementation fault |
| `tag-refused` | a pimdir tag names no flag, or adds and removes one flag; nothing is staged |

#### Scenario: A body not downloaded yet
- GIVEN a pimdir item whose body is not local
- WHEN `himalaya --json message read` reads it
- THEN the command exits 1 and prints `{"code":"body-pending","error":"…","sources":[],"backtrace":null}`

### Requirement: Plain output is printable
A single-line field taken from a message or a server (a subject, a name, a filename, a mailbox name, an id) SHALL have its control characters (C0, DEL and C1) and bidi controls (U+202A to U+202E, U+2066 to U+2069) replaced with U+FFFD before it is printed as plain output, so it can neither drive the terminal nor reorder what is displayed. The designed `message read` and `message parse` views SHALL replace those characters in decoded bodies too, retaining tabs and line feeds and normalizing CRLF to LF, on terminals and redirected stdout alike. JSON strings and explicitly requested raw bytes SHALL remain unchanged. Other byte writers SHALL retain their binary-content guard on terminals.

### Requirement: Generation commands print to the standard output
The `completion`, `manual` and `json-schema` commands SHALL share one shape: a positional list selecting what to generate, defaulting to everything, and an optional `--dir` deciding where it lands. Without a directory they SHALL print the single selected item to the standard output, so a packaging helper can capture it and a shell can redirect it to a file, and SHALL fail when several items are selected rather than concatenating pieces valid for nothing. With a directory they SHALL write one file per selected item in it, creating the directory when missing, and report where each landed.

### Requirement: Data commands serialize their data
Every command SHALL hand the printer a dedicated output type implementing both `Display` and `Serialize`, and register its JSON Schema under the command's invocation key; a sentence is the text form of that data, never a JSON field. Where a sibling `list` already serializes a backend resource, the `get` output SHALL emit that resource verbatim through a transparent newtype, so one item read with `get` has the shape of one row of `list`. Where the wire type is unsuitable (a recursive MIME tree, a type carrying no schema), the output type SHALL name its fields instead. `gmail threads get` is the exception for the MIME tree: `users.threads.get` has no raw format, so each message's `payload` SHALL be serialized when the requested format carries one. A Gmail settings write SHALL print the settings the server returns, in the shape of the matching `get`. A Gmail list SHALL print the provider's collection under its own key (`delegates`, `filter`, `forwardingAddresses`), and `gmail messages list` its `messages`.

### Requirement: Serialized collections are always present
An output field holding a collection SHALL be serialized even when empty, because the schema marks it required regardless and a skipped field would contradict the published schema.

### Requirement: JSON keys are camelCase
Every output type handed to the printer SHALL serialize its keys as camelCase, matching the wire formats the backends speak (JMAP per RFC 8620, Microsoft Graph, the Google APIs) and keeping every key reachable by dot access in jq and JavaScript. Each one SHALL be named `<Domain><Target><Verb>Output`, the domain omitted for the shared commands. A field carrying a provider spelling verbatim SHALL keep its explicit `#[serde(rename)]`, since `@odata.nextLink` and `nextPageToken` are the provider's names rather than derivable ones, and a transparent newtype over an io-gmail or io-msgraph resource SHALL serialize that resource as its crate declares it. The rule covers keys only: string values keep their own spelling (error codes, flags, mailbox roles, `message delete` actions and pimdir queue states stay kebab-case or lowercase). Configuration types SHALL stay kebab-case: they are read from TOML and are not `--json` payloads.

### Requirement: Gmail header selection applies to every format
`gmail messages get --header` and `gmail threads get --header` SHALL narrow the rendered headers whatever the requested format. Gmail honours its `metadataHeaders` parameter under the metadata format alone and returns every header otherwise, so the narrowing SHALL also be applied to the response. Matching is case-insensitive, order and repeats are preserved, and passing no `--header` renders every header. Headers are read from the top-level payload part, where Gmail puts the RFC 5322 headers.

### Requirement: Raw message formats write bytes
A Gmail `get` command asked for the raw format SHALL decode the fetched message and write its RFC 5322 bytes through the shared byte writer rather than rendering a summary. This covers `messages get` and `drafts get`, and `msgraph messages get --raw`. The byte writer SHALL write the bytes verbatim in text mode and print `{data, size, mime}`, `data` in base64, under `--json`, so no command writes raw bytes to a JSON stdout; given a path, it SHALL write the file and print `{path, size}`. `gmail attachments get`, `msgraph attachments get` and `jmap email export` SHALL print through it.

### Requirement: One module per subcommand
A protocol command resource SHALL live in a sibling module file carrying its `pub mod` declarations and its `Command` enum, next to a folder holding one file per subcommand. Types live with the subcommand that owns them; a type serving several subcommands lives in the file of the one that owns it, or in its own module when it belongs to none.

### Requirement: Command types carry a domain and a target
A protocol command type SHALL be named `<Domain><Target><Verb>Command`, so a bare verb is never a type name. The domain prefix stays wherever the bare name would collide across the backends a CLI spans, and is otherwise omitted for the tables and value enums under the cli subtree.

### Requirement: Composers default the From header
`messages compose`, `messages reply` and `messages forward` SHALL fill the `From` header from the resolved account when `--from` is not passed: `email` as the address, `display-name` as the name it carries. An explicit `--from` SHALL win whole, the configured name never being grafted onto an address the user spelled out. With neither, the header SHALL be omitted rather than guessed.

The name SHALL be handed to the MIME builder apart from the address, so that a name carrying a comma, a quote or a non-ASCII character is encoded by the builder rather than by a quoting rule of Himalaya's own.

The composers SHALL write the `Date` (the current time at the local offset) and the `Message-ID` themselves, mime-rs's builder writing no field it is not given: the id sits on the domain of the `From` address when it is a plain dot-atom, else on the machine's hostname, else on `localhost`, its left part unique on the machine.

The same composers SHALL end the body with the account's `signature`, introduced by its `signature-delim`, when neither `--signature` nor `--signature-file` is passed. `--signature` SHALL win, and `--signature-file` SHALL win too, the configured signature standing down rather than shadowing the file the flag names.

With `--json` and neither `--save` nor `--send`, the same composers SHALL print the decoded `from`, `to`, `cc`, `bcc`, `subject` and `body` instead of the raw bytes, the body without any signature, so an editor can lay the message out and hand it back through the flags, sending appending the signature once. `--posting-style none` SHALL send the written body alone, the source left unquoted, for a writer who already laid the quote out.

### Requirement: Reply to all recipients
`message reply --all` (`-A`) SHALL keep the `To` of a plain reply and fill `Cc` with the `To` and `Cc` of the source, without the account's `email` and without any address already in `To`, each address once, compared ignoring case, display names kept. An explicit `--to` SHALL replace the derived `To`, the `Cc` then being deduplicated against it, and an explicit `--cc` SHALL replace the derived `Cc` whole. Without `--all`, `Cc` SHALL stay empty unless `--cc` fills it.

### Requirement: Raw message input is shared
A command taking a raw RFC 5322 message SHALL resolve it through the shared `MessageArg`: a file path, an inline value after `--`, or piped stdin read whole. The message SHALL be kept as bytes, never decoded, so 8-bit content in any charset reaches the backend intact. The resolved message is normalised to CRLF and rejected when empty, so no backend receives a zero-length message.

### Requirement: Gmail draft writes return identities
`gmail drafts create` and `gmail drafts update` SHALL serialize the returned draft `id`, and nullable `messageId` and `threadId`, under `--json`, with schemas registered for both commands. Non-JSON output SHALL retain the existing success sentence. A response without a message SHALL still succeed, with `messageId` and `threadId` null, since the draft was already written.

### Requirement: Gmail history retains added-message details
`gmail history list --json` SHALL retain each added message's `id`, nullable `threadId`, and supplied `labelIds` in a `messagesAddedDetails` array, preserving order and duplicate entries. Missing labels SHALL produce an empty array. The existing `messagesAdded` id array, other change arrays, pagination, and text counts SHALL remain unchanged. The registered JSON Schema SHALL describe the details array, which SHALL remain present when empty.

### Requirement: IMAP fetch can return whole messages
`imap fetch --body` SHALL add `BODY.PEEK[]` (RFC 3501 §6.4.5) to the requested data items, so a caller downloads every message of a sequence set over one session instead of one `message read` per message. The fetch SHALL NOT set `\Seen`. The JSON output SHALL carry the message octets byte-exact as the standard Base64 of the `body` field, which the JSON Schema SHALL declare with `contentEncoding: base64` and `contentMediaType: message/rfc822`; the plain rendering SHALL print the size only. `--body` SHALL NOT imply `--envelope`.

### Requirement: Saving a sent message follows the send
A command both saving and sending (`--save` with `--send`, `message send --save`, `message add --send`) SHALL send first and save afterwards, so a failed send leaves no copy. A save failing after a successful send SHALL fail the command with an error stating that the message was sent.

### Requirement: A sent reply or forward flags its source
`message reply --send` SHALL add the `answered` flag (RFC 3501 §2.3.2) to the source message once the send has succeeded. A flag failing after a successful send SHALL fail the command with an error stating that the message was sent. `message forward --send` SHALL add the `forwarded` flag (RFC 5788) to the source the same way, best-effort: a backend refusing the keyword SHALL be logged as a warning and SHALL NOT fail the command. A reply or forward only printed or saved SHALL leave the source untouched.

### Requirement: A forward carries the attachments of its source
`message forward` SHALL append every attachment part of the source (multipart containers excluded) after the quoted body, under its file name, with its MIME type and decoded bytes, the quoted body composing as it does without them. A nameless `message/rfc822` part SHALL be named from its subject, sanitized for the filesystem, plus `.eml`, or `attachment.eml`; any other nameless part SHALL be named `attachment` plus the extension of its MIME type (`.bin` for `application/octet-stream`), or bare `attachment`. `--no-attachments` SHALL carry none and combines with `--attach`. A reply SHALL never carry the attachments of its source.

### Requirement: A read message is a designed view
`message read --json` SHALL print `{headers, text, html, parts}`, built from the raw message so it is the same on every backend, rather than the parser's own model. `headers` SHALL carry `from`, `to`, `cc`, `bcc`, `replyTo` and `sender` as lists of `{name, email}` (groups flattened, an entry without a valid email dropped, `name` decoded or `null`), `subject` decoded, `date` as RFC 3339 with its offset or `null`, `messageId` without angle brackets, `inReplyTo` and `references` as lists without angle brackets, as envelopes print them, and `listId`, `listUnsubscribe`, `precedence` and `autoSubmitted` as found, decoded. `text` SHALL be every `text/plain` body part decoded to UTF-8 by its charset, lossily, a `format=flowed` part unwrapped (RFC 3676 §4.2), joined by a blank line, and `html` the first `text/html` body part, each `null` when absent. The message SHALL be parsed by mime-rs, whose structure is the one RFC 2046 delimits: a line opening with a boundary but carrying more than transport padding is no delimiter, an outer delimiter closes the parts nested in it, and a field whose name holds a blank (`Sub ject:`) is not the field it spells. The text output SHALL show the same headers, parts, sizes and names as the JSON view. `--raw` SHALL write the message bytes verbatim, and print `{data, size, mime}` under `--json`.

### Requirement: Malformed mail reads as its sender meant
The view SHALL read a header from its raw bytes: raw 8-bit bytes SHALL be decoded as UTF-8 when they are valid UTF-8 (RFC 6532) and as Windows-1252 otherwise, and adjacent RFC 2047 encoded words of one charset, separated by linear whitespace alone, SHALL be joined as bytes before their charset is decoded, the whitespace between adjacent encoded words being dropped (RFC 2047 §6.2). This applies to the subject, the display names, the list headers and the file names; an RFC 2231 file name SHALL win over a plain one. A text part labelled `us-ascii` or `ascii`, or naming no charset, SHALL be read as UTF-8 when its bytes are valid UTF-8 and as Windows-1252 otherwise, in `text` and `html` alike. The charsets are the WHATWG Encoding Standard's: UTF-7 (RFC 2152), outside it, SHALL be read as written. A `Content-Type` whose type or subtype is missing, empty or not an RFC 2045 token SHALL be taken as `text/plain; charset=us-ascii` (RFC 2045 §5.2), its part reporting that `mime` and `charset`.

### Requirement: The read view lists unreadable address entries
`headers.invalid` SHALL be an object `{to, cc, bcc, replyTo, from, sender}`, each a list of the entries of that header that are not readable as an address and so are absent from its list, as decoded text (encoded words decoded, trimmed), in header order, empty when none. A group name SHALL NOT be an entry, nor an empty group. `message read --json` and `message parse --json` SHALL print the same.

### Requirement: Every part has an id and a role
`parts` SHALL list every leaf part, depth first, as `{id, role, mime, filename, size, contentId, charset, method}`, the `id` being the one `attachment list` and `attachment download` take: the 1-based position of the part in the whole part list. A `multipart/*` SHALL be no part; `message/rfc822` or `Content-Disposition: attachment` SHALL be an `attachment`; a `text/plain` or `text/html` without a filename SHALL be a `body`; an `image/*` marked inline or carrying a `Content-ID` SHALL be `inline`; anything else SHALL be an `attachment`. `filename` SHALL be RFC 2231 and RFC 2047 decoded, `null` when the part names none; `size` SHALL count the part's bytes once its transfer encoding is undone, in its own charset; `contentId` SHALL carry no angle brackets; `method` SHALL be a `text/calendar` part's `method` parameter, uppercased. `attachment list --json` SHALL report the same ids, sizes, `contentId` and `role`, and `path`, where `attachment download` wrote the part, `null` until then.

### Requirement: A message too complex is refused whole
A message nested deeper than 8 levels, or holding more than 200 parts or 500 header lines (folded lines and the headers of every part and attached message included), SHALL be refused with the JSON error code `message-too-complex` rather than read in part, by `message read` (except `--raw`) and the `attachment` commands.

### Requirement: A part can be read without touching the disk
`attachment download <MESSAGE-ID> <PART-ID> --stdout` SHALL take exactly one part id, any leaf part `message read` lists, and write that part's decoded bytes to stdout with nothing else, touching no file; `--dir` SHALL be refused beside it. Under `--json` it SHALL print `{id, mime, filename, size, data}`, `data` in base64. Without `--stdout`, the attachment parts SHALL be written to the download directory as the same decoded bytes.

### Requirement: A message can be read without an account
`message parse [EML]` SHALL read a raw message from a path, or from stdin with `-` or when omitted, and SHALL print what `message read` prints for the same bytes. It SHALL resolve no account and read no configuration. A source holding nothing but whitespace SHALL be refused, naming where it was read from; the bounds and the `message-too-complex` code SHALL apply as on `message read`.

### Requirement: A parsed message hands one part over
`message parse --part <PART-ID>` SHALL write that part's bytes, transfer encoding undone, to stdout, or under `--json` `{id, mime, filename, size, data}` with `data` in base64; the ids SHALL be those of the view. An unknown id or a container's SHALL be refused.

`message parse --section <SECTION>` SHALL do the same for the leaf part at that IMAP section (RFC 3501 §6.4.5): `1` for the body of a message that is not multipart, else each part's 1-based position among its siblings, joined with dots down the message's own multiparts. The JSON `id` SHALL stay the view's. A container's section, a part of an attached message, or a section naming nothing SHALL be refused; `--part` and `--section` SHALL NOT be given together.

### Requirement: Downloaded attachments never replace an existing entry
Attachment downloads SHALL atomically create a new file, refusing to follow an existing leaf symlink. An occupied name SHALL be suffixed with a number before its extension. If the original name and suffixes 1 through 1023 are occupied, the command SHALL fail rather than overwrite a file. Concurrent downloads SHALL reserve distinct names.

### Requirement: A command given message ids says how many it found
`flag add`, `flag set`, `flag remove`, `message move`, `message copy` and `message delete` SHALL fail, exit code 1, when none of the given ids exists in the mailbox, changing nothing. When some exist they SHALL succeed and report how many as `count`: the flag commands beside `flags`, the `{raw, iana}` flags written, `message move` and `message copy` beside `queued`, `message delete` beside its `action`. `message delete` on a message already in the trash is not covered. The plain output of these commands SHALL end with a newline.

#### Scenario: An id that does not exist
- GIVEN a mailbox holding no message with id `99999999`
- WHEN `himalaya flag add --flag flagged 99999999` runs
- THEN the command exits 1 with `No flag added: no id matched in the mailbox`

#### Scenario: One id of two exists
- GIVEN a mailbox holding message `42` and no message `99999999`
- WHEN `himalaya --json flag add --flag flagged 42 99999999` runs
- THEN the command exits 0 and prints `{"flags":[{"raw":"\\Flagged","iana":"flagged"}],"count":1,"queued":false}`

### Requirement: A queued write says it is queued
On a backend that queues its writes for a sync engine to apply later (pimdir), the receipt of a shared write SHALL carry `queued: true`, and its text form SHALL say the write is queued rather than done: `message send`, `message add`, a composer's save or send, `message delete`, `message move`, `message copy`, `flag add|set|remove` and `mailbox create|rename|delete`. A mailbox write SHALL print its `id` `null` while queued, and no queue detail (a row, a queue id) SHALL reach a shared output. Every other backend SHALL print `queued: false`.

#### Scenario: A send on a pimdir account
- GIVEN a pimdir account with `mailbox.alias.sent` set
- WHEN `message send` runs
- THEN it prints `Message queued for sending`, not that the message was sent

#### Scenario: A delete on a pimdir account
- GIVEN a pimdir account with a trash mailbox
- WHEN `message delete 3` runs on another mailbox
- THEN it prints `Queued the move of 1 message(s) to the trash`

### Requirement: Shared mailbox writes
`mailbox create <NAME> [--parent <MAILBOX>]` SHALL create a mailbox named NAME, one level of the hierarchy, at the top of the account or under the parent; `mailbox rename <MAILBOX> <NAME>` SHALL replace the last level of the mailbox's name with NAME, its parent and messages kept; `mailbox delete <MAILBOX>` SHALL delete the mailbox and the messages it holds, the mailbox named with no default. Mailboxes are given by name, alias, role or id. A backend naming the hierarchy by path SHALL join the levels with its own delimiter: the one IMAP's `LIST` reports, `/` on Gmail and the local stores. The outputs SHALL be `{id, name}` for create and rename, `id` being the mailbox's id afterwards (a rename changes it where the name is the id) and `null` when the write is queued, and `{id}` for delete. A write SHALL drop the mailbox index a backend caches, so a later command of the session resolves the new name.

#### Scenario: A Maildir mailbox under a parent
- GIVEN a Maildir account holding `Archive`
- WHEN `mailbox create 2025 --parent Archive`, then `mailbox rename Archive/2025 2026`, then `mailbox delete Archive/2026` run
- THEN `Archive/2025` is created, renamed to `Archive/2026`, then removed

### Requirement: The important and junk flags are shared
The shared flag arguments SHALL include `important` (`$Important`, RFC 8457) and `junk` (`$Junk`), each its capability, `flags.important` and `flags.junk`.

### Requirement: The auto-reply is shared
`vacation get` SHALL print the account's auto-reply as `{enabled, from, to, subject, text, html}`, the dates RFC 3339 in UTC, and the default one, off with nothing set, when the backend holds none. `vacation set` SHALL change only what it is given (`--enable` or `--disable`, `--from`, `--to` as RFC 3339 or a `YYYY-MM-DD` read as midnight UTC, `--subject`, `--text`, `--html`) and print the auto-reply as it then stands. JMAP serves it when its session advertises `urn:ietf:params:jmap:vacationresponse` (RFC 8621 section 8), Gmail through `settings.getVacation` and `updateVacation`, its dates epoch milliseconds, the settings read first so the Gmail-only ones survive.

### Requirement: An account lists what it serves
`account capability` SHALL print one row per shared capability, `{name, backend, level, detail}` under `{account, capabilities}`: the backend the resolver picks for the account and `--backend`, `null` when none is configured, and that backend's level and detail. It SHALL read the configuration and the capability tables and connect nowhere. On a pimdir account, a write SHALL be read from what the store's sources declare: an item write through the store's gate in each of the account's mail collections (`none` refused in all, `partial` refused in some or passing in part), an intent served when a source performs it, the static row kept when no source declares anything.

#### Scenario: A Maildir synced from JMAP
- GIVEN an account configuring a Maildir and a JMAP block
- WHEN `account capability` runs
- THEN `mailbox.list` names `maildir`, `vacation` names `jmap` at `server`, and nothing connects

### Requirement: A write prints what it acted on
Every write SHALL print what it acted on, never a sentence alone. A shared write SHALL print its receipt with `queued`: `message copy` and `message move` `{count, queued}`, `message send` and a composer's save or send `{id, saved, sent, queued}`, `id` being the saved copy's, `null` when the message was not saved, the save is queued, or the backend returned none. A composer neither saving nor sending SHALL print the template under `--json` and the message bytes otherwise. A protocol write with no payload of its own (Gmail, Microsoft Graph, JMAP, maildir, mbox, m2dir, ManageSieve) SHALL print `{id}` naming the item it acted on as it is afterwards, or `{ids}` for several, in the order acted on; its text form SHALL keep the sentence. `jmap email import` SHALL print `{id, blobId}`, `id` `null` with `--upload-only`; `msgraph messages send` `{id: null}`, `sendMail` answering without one; `smtp send` the envelope it sent, `{mailFrom, rcptTo}`.

#### Scenario: A Gmail label is created
- GIVEN a Gmail account
- WHEN `himalaya --json gmail labels create Projects` runs
- THEN it prints `{"id":"Label_12"}`, the id Gmail gave the label

### Requirement: Absent is null
An output type of Himalaya's own SHALL print every key, `null` when the value is absent, so its schema marks every key required. A provider payload printed as received SHALL keep its own omissions.

### Requirement: Listings resume from their cursor
`envelope list` and `envelope search` SHALL print `nextPage` beside `envelopes`: the next page number, as a string, while a page comes back full, `null` otherwise, and SHALL take it back through `--page-token`, which wins over `--page`. A full last page leads to an empty one. `msgraph messages list` SHALL print the `$skip` of Graph's next link as `nextPage` and take it back through `--page-token`; a next link carrying no `$skip` (a `$search` page) SHALL be printed whole and refused by `--page-token`. The Gmail listings keep Gmail's page token.

#### Scenario: A second page
- GIVEN a mailbox holding two messages
- WHEN `himalaya --json envelope list -s 1` runs, then `himalaya --json envelope list -s 1 --page-token 2`
- THEN the first prints one envelope and `"nextPage":"2"`, the second the other envelope

### Requirement: An IMAP command prints its completion
Every `imap` command but `raw` SHALL print the tagged completion of its exchange: `status: "OK"` beside what the command understood of the server's answer, or `{status, text}` when the server answers `NO` or `BAD`, exiting 2 after it. A command getting no completion (a network, TLS or parse failure, a local error) SHALL fail with the failure object. `select` SHALL print the mailbox's state (`flags`, `exists`, `recent`, `unseen`, `permanentFlags`, `uidNext`, `uidValidity`, `highestModSeq`), `expunge` the expunged message numbers, `store` the FETCH data the server returns, `append` the `APPENDUID` it returns (UIDPLUS), `copy` and `move` the `COPYUID`, `null` and empty without UIDPLUS, the mailbox writes `{mailbox}` and `close` and `unselect` the status alone. `fetch` SHALL print envelope addresses as `{name, email}`, and `search` its ids as numbers, as `sort` does. io-imap returns a typed exchange's output whole, so these commands print no events.

#### Scenario: A mailbox that does not exist
- GIVEN an IMAP account
- WHEN `himalaya --json imap select Nope` runs
- THEN it prints `{"status":"NO","text":"…"}` with the server's text and exits 2

### Requirement: A local store flag prints its meaning beside its spelling
`maildir flag list` and `mbox flag list` SHALL print each flag as `{raw, iana}`, `raw` being the store's letter and `iana` the keyword it stands for, `null` for mbox's `O`, which has none; mbox rows SHALL also name the `header` carrying the letter.

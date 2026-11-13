# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added `--no-from`, `--no-to`, `--no-subject`, `--no-text` and `--no-html` to `vacation set`, clearing a field of the auto-reply (JMAP sends `null`, Gmail unsets it), so an auto-reply can go back to having nothing set.
- Added the capabilities an argument reaches to the `Backends:` footer of `--help` (`submit (with --send, --no-save)`, `message.add (with --save)` on `message compose`, `reply` and `forward`, `compose` getting its footer), and left out of an MCP tool the argument the account cannot serve (`send` on an account with no transport), the tool kept.
- Added the first-run welcome: run at a terminal without a configuration, bare `himalaya` and every command reading one print on stderr the missing path, the command creating it from config.sample.toml (copied from share/doc/himalaya/ when one ships beside the binary, downloaded with `curl` otherwise) and the first command to run, `himalaya envelope list`, then exit 1. Under `--json` or with stderr piped, the error names the path and the sample instead. A bare `himalaya` prints the help when a configuration exists.
- Added config.sample.toml to the Nix package and the release archives as share/doc/himalaya/config.sample.toml, which install.sh copies under its prefix and the first-run welcome copies from.
- Added `attach-data` to the MCP tools `message_compose`, `message_reply` and `message_forward`, attaching files given inline: a list of `{filename, mimeType, data, text}`, with `data` in base64 or `text` as UTF-8, `mimeType` guessed from the filename as `--attach` guesses it ([#806]). It is an argument of the server alone, not of the CLI, where `--attach` already takes a path. Each attachment over `himalaya mcp --max-attachment-size` (10 MiB by default) is refused before it is decoded, under the new error code `attachment-too-large`; a malformed one is refused under `attachment-invalid`. Under `--allow-send`, an agent can now send arbitrary binary data, not only text.
- Added `--page-token` to `envelope list` and `envelope search`, which print `nextPage` (the next page number while a page comes back full, `null` otherwise), and to `msgraph messages list`, `msgraph mail-folders list` and `msgraph mail-folders child-folders`, whose `nextPage` is the `$skip` of Graph's next link, `null` when the link leads back to the page asked (`--top 0`); a `$search` next link carrying no `$skip` prints whole, as `Next link:` in text.
- Added a JSON Schema for every command, 216 in all, checked by a test walking the command tree, with `himalaya-imap-raw.events` and `himalaya-sieve-raw.events` describing the events those commands print.
- Added events to the MCP tool results, one text content each before the result; a refused protocol completion or a failed `account check` keeps its result as `structuredContent` with `isError`.
- Added `attachment_download` to `himalaya mcp`, a reading tool always run as `attachment download --stdout` and taking one part id, so an agent reads an attachment without a file being written ([#805]). Its structured content is the part without `data` (`{id, mime, filename, size}`), with that schema as output schema, and the bytes follow once as an embedded resource (`blob` in base64, the part's MIME type, the URI `himalaya://<account>/<mailbox>/<message-id>/<part-id>`). The server flag `--max-attachment-size` added with `attach-data` also refuses a larger part before it is encoded, under `attachment-too-large`.
- Added `mailbox create <NAME> [--parent <MAILBOX>]`, `mailbox rename <MAILBOX> <NAME>` and `mailbox delete <MAILBOX>` to the unified API, on every backend: a name is one level of the hierarchy, a rename keeps the parent, a delete takes the messages the mailbox holds. IMAP joins the levels with the delimiter its `LIST` reports, Gmail and the local stores with `/`. On pimdir each is queued, its `id` `null`. Graph creates under a parent in two requests; what a delete leaves behind on JMAP, Gmail and Graph is in the `--help` footer.
- Added the `important` and `junk` flags to `flag add|set|remove` (RFC 8457 `$Important`, `$Junk`). Graph has no junk mark and refuses it; on Gmail `junk` is the `SPAM` label, which moves the message to Spam.
- Added `vacation get` and `vacation set`, reading and changing the auto-reply (`{enabled, from, to, subject, text, html}`, dates in RFC 3339 UTC) on JMAP and Gmail. It resolves to the first configured backend keeping one, so an account syncing a Maildir from JMAP reaches JMAP. IMAP refuses it naming `himalaya sieve`; the MCP server lists `vacation_set` under `--allow-send` only, an auto-reply answering mail.
- Added `account capability`, listing what the account serves of the unified API, one row per capability: the backend it resolves to (`null` when none is configured), its level (`full`, `partial`, `server`, `none`) and the detail, under `--backend` when given. It connects nowhere; on a pimdir account the writes are read from what the store's sources declare. Frontends read it to hide what the account cannot run.
- Added a capability table per backend, naming each unified capability in pimdir's grammar (`message.copy`, `flags.answered`, `submit`, `envelope.search.attachment`) at a level, `full`, `partial`, `server` or `none`, with a detail saying what differs or why. A unified command reaching a capability its backend declares `none`, or one no configured transport answers, is refused before connecting, under the new error code `capability-missing`, naming the account, the capability, the backend and what serves it instead.
- Added a generated `Backends:` footer to the `--help` of every unified command, listing per capability it reaches the backends at each level and the details.
- Added the `before <date>`, `cc <pattern>`, `bcc <pattern>`, `address <pattern>` (the `From`, `To`, `Cc` and `Bcc` headers at once) and `attachment` clauses to the `envelope search` query, on every backend that can answer them: IMAP refuses `attachment`, SEARCH having no key for it.
- Added the error codes `search-invalid`, for a query that does not parse, and `search-unsupported`, for a clause the account's backend cannot search, which names the clause as written: every flag on Microsoft Graph, `flag answered` and keywords on Gmail, `attachment` on IMAP. Nothing is searched and no clause is dropped.
- Added the error codes `message-not-found`, carried by every refusal of ids none of which the mailbox holds (`flag add|set|remove`, `message copy|move|delete`, `pimdir flag add|set|remove`) and by the raw `maildir` id-taking commands given one unknown id, and `mailbox-not-found`, for a Maildir, an mbox or an IMAP mailbox that does not exist (``Maildir mailbox `X` not found``, ``Mbox mailbox `X` not found``, ``IMAP mailbox `X` not found``). The raw `mbox` id-taking commands given one unknown id, and a single message read whose id IMAP or mbox does not hold, carry `message-not-found` too. Nothing is done.
- Added `maildir.layout`, `fs` by default or `maildir++` for a Dovecot or Courier store: the root is the inbox, `.a.b` lists as `a/b` and is taken under either spelling, and a level holding a dot is refused. A rename or delete takes the `.a.*` children of `a` along, as the fs layout does (io-maildir).
- Added `himalaya mcp`, a Model Context Protocol server (revision 2025-06-18) over stdio for one account, exposing the unified API as tools named after their command (`envelope_list`, `message_read`), run over clients kept for the server's lifetime as in `himalaya repl`. `initialize` gives the agent instructions: how to find a mailbox and an envelope id, the query language of `envelope_search`, attachments read as embedded resources and given inline, the flags gating writing and sending. A tool's input schema is generated from its command, its arguments spelled as on the command line, and an argument it does not take is refused naming those it does; a result is the command's `--json` payload as structured content, with its JSON Schema as output schema where the payload has one shape, and a failure, a command printing no value or several included, is a tool error carrying the error object and its code. Each line is one JSON-RPC 2.0 object: a batch, a value that is no object or a request without `"jsonrpc": "2.0"` answers -32600, a line that is not UTF-8 -32700, and the server reads on. Only the reading tools are listed by default: `--allow-write` adds those changing the store and `--allow-send` those sending mail. `message parse` and the file arguments are not exposed, and a message given to a tool is never read from a path it spells. A server failing to start, its configuration missing included, says why on stderr, stdout carrying protocol messages only.
- Added `himalaya repl`, a session over stdin and stdout for one account: each line is a command without the binary name, split as a shell splits it and run over clients built the first time a command needs them and kept for the session, so a client connects once rather than once per command. A blank line or a `#` comment is skipped; `quit`, `exit` or EOF ends it. The account, the configuration, the logs and `--backend` are the session's, a line setting `-a`, `-c`, `--log-level` or `-b` being refused naming the flag; `--json` on `repl` switches the session to JSON, on a line that line alone. Piped, every line gets exactly one answer on stdout, a failure printed as the error object, the help going to stderr in a JSON session, and the loop goes on after a parse or command error. A command that fails drops the client it ran on, which the next command builds again. A command that would read its input from stdin inside a session is refused with the new error code `stdin-taken`, and `message compose`, `reply` and `forward` read no body from it. `repl` and `mcp` are refused inside a session.
- Added `pimdir search <QUERY>...`, searching the account's mail in the pimdir query language (pimdir SEARCH.md §8: words, phrases, `field:value` terms, `and`, `or`, `not`, parentheses) from the store's search index, which the sync engine builds and refreshes; Himalaya only reads it. `--sort date|name|relevance`, `--page`, `--page-size` and `--threads`; each hit carries its envelope, `mailboxes`, what `matched` (`body` or `summary`) and a `snippet`, and every answer a `coverage` (`bodiless`, `unindexed`, `behind`, `since`, `until`, `filling`, `complete`), printed as a line under the list when incomplete. A queued write shows in no answer. Its years, months, days, `today` and `yesterday` are read in the local zone, `TZ` then the system's, else UTC.
- Added `pimdir count <QUERY>...`, counting the account's mail a query matches, with its coverage.
- Added `pimdir thread <ID>`, listing the thread of a message from the search index, oldest first.
- Added `from`, `date` and `flags` to each `imap thread` entry, the envelope of every threaded message being fetched with the thread.
- Added `pimdir tag <QUERY>... --add <FLAG> --remove <FLAG>`, queueing one flag change per mailbox holding each matched message, every change checked against the sources before the first is queued; `--dry-run` queues nothing. A message whose flags were never synced, or whose mailbox several sources or none sync, is reported and left alone.
- Added `pimdir flag list|add|set|remove`, reading and staging any flag of a pimdir message by its raw spelling, keywords included, as `imap store` does on IMAP ([#790]). A registered flag is taken in its canonical spelling only (`\Seen`, not `seen`); `\Deleted`, `\Recent`, a backslash before any other word, non-ASCII characters and the characters IMAP forbids in a flag (`(`, `)`, `{`, `%`, `*`, `"`, `]`, `\`) are refused; and a source declaring no keyword support refuses a keyword change, one supporting them in part staging it with its warning. The writes print the queue row of each message, `null` for a message the change leaves as it was, for which nothing is staged, and fail when no given id exists; `list` prints `null` for flags the sync engine never read.
- Added `msgraph messages send-draft <ID>`, sending an existing Microsoft Graph draft (`POST /me/messages/{id}/send`), which Graph moves to Sent Items ([#787]).
- Added `--no-categories` to `msgraph messages update`, removing every category from the message, which `--category` could not do.
- Added `--all` (`-A`) to `message reply`, replying to everyone: `To` stays the source's `Reply-To` or `From`, and `Cc` takes the `To` and `Cc` of the source without the account's `email` nor anyone already in `To`, each address once. An explicit `--to` or `--cc` takes precedence over the list it would derive ([#779]).
- Added `pimdir attachment list`, one page of the files a pimdir account's messages attach, newest message first, read from the stand-ins the store records as each body enters it (pimdir STORAGE §14.3): each with its `id`, `name`, `mediaType`, `size` and IMAP `section`, and the `messageId`, `mailbox`, `sender`, `senderName` and `date` of its message, the ids being those `envelope list` and `message read` take, plus a `nextPage` token that `--page-token` takes. `pimdir attachment search <WORDS>` keeps the files every word of which is in the file name, the sender's address or the sender's name, ignoring case.
- Added `--section <SECTION>` to `message parse`, writing the part at that IMAP section (`2`, `1.2`) as `--part` writes a part by its id: what a pimdir store records of an attachment.
- Added `pimdir mailbox list`, listing each mailbox of a pimdir account with its `total` and `unread` counts, its `coverage` (`{since, until, at}`: the scope the sync engine last listed it within, `null` until every source closed a round on it) and the `round` under way (`{since, until, startedAt}`), so a reader can say which mail the store holds (pimdir scoped-mail-sync).
- Added `message parse [EML]`, printing the `message read` view (the same JSON or text) of a raw message read from a file or stdin, with no account and no configuration, and `--part <PART-ID>`, writing one part's decoded bytes as `attachment download --stdout` does. A blank source is refused, and the `message-too-complex` bounds apply.
- Added a designed `--json` view to `message read`, the same on every backend: `headers` (address lists as `{name, email}`, decoded subject, RFC 3339 date, ids without angle brackets, list headers), `text` and `html` bodies decoded to UTF-8, and `parts`, every leaf part with its id, role (`body`, `inline` or `attachment`), type, decoded file name, size, `contentId`, `charset` and calendar `method`. A message nested deeper than 8 levels, or holding more than 200 parts or 500 header lines, is refused with the error code `message-too-complex`.
- Added `--stdout` to `attachment download`, writing the decoded bytes of one part to stdout and no file, or under `--json` printing `{id, mime, filename, size, data}` with `data` in base64.
- Added `contentId` to `attachment list --json`.
- Added `headers.invalid` to the `message read` and `message parse` views: for `to`, `cc`, `bcc`, `replyTo`, `from` and `sender`, the entries that are not readable as an address and so are absent from the lists, as decoded text in header order (group names and empty groups excluded).
- Added to the `message read` view the reading of malformed but common mail: raw 8-bit header bytes as UTF-8 when valid and Windows-1252 otherwise (subject, names, list headers, file names), adjacent encoded words of one charset joined as bytes so a character split between them is rebuilt, UTF-8 bytes labelled `us-ascii` read as UTF-8, and an invalid `Content-Type` such as `text/` taken as `text/plain; charset=us-ascii`.
- Added `message.reply.posting-style` and `message.forward.posting-style`, global or per account, the posting style used when `--posting-style` is not passed ([#772]).
- Added the role of a pimdir mailbox: `mailbox list` shows the role the sync engine recorded from the server (`collections.role`), and a role name (`inbox`, `sent`, `drafts`, `trash`, …) addresses its mailbox wherever a mailbox is taken, as on JMAP, so `envelope list` without `-m` reads the inbox and `message delete` finds the trash without `mailbox.alias.trash`. A collection's name does not address it.
- Added a stable `code` to the `--json` error output for failures a caller is expected to act on, found anywhere in the error chain. The first is `body-pending`: a pimdir message listed but whose body is not downloaded yet. A pimdir store failure carries the code io-pimdir gives it: `store-version`, `store-uncreated`, `store-foreign`, `store-corrupt`, `store-not-current`, `store-hash-algo`, `sync-version`, `store-busy`, `store-owned`, `store-staging`, `store-in-flight`, `store-exposed`, `store-reset`, `lock-replaced`, `item-duplicate`, `flag-invalid`, `action-malformed`, `conflict-unread`, `unsupported`, `no-performer` or `ambiguous-performer`, and a pimdir search failure `index-missing`, `index-outdated`, `index-version`, `index-zone-unknown`, `index-fts5-missing`, `query-refused`, `query-unanswerable` or `tag-refused`.
- Added `pimdir message add` and `pimdir message send`, staging as the unified commands do and printing the queue row with the `Message-ID`, and `pimdir queue show <ROW>`, saying whether a row is pending, parked (with why), applied (with the id of the message an add created; a message sent or a mailbox created once the sync engine acknowledges it) or unknown.
- Added `pimdir mailbox create <NAME>`, queueing a `collection-create` intent for the sync engine to create the mailbox on its server.
- Added `pimdir mailbox rename <MAILBOX> <NAME>`, queueing a `collection-rename` intent for the sync engine to rename the mailbox on its server, keeping its parent and its messages (pimdir STORAGE Annex B.2, capability `collection.rename`).
- Added `pimdir mailbox delete <MAILBOX>`, queueing a `collection-delete` intent for the sync engine to delete the mailbox on its server with the messages it holds (capability `collection.delete`). A mailbox holding other mailboxes is never deleted with them: the row parks naming them. Once the deletion reaches the store, `pimdir queue show` reads the row as `unknown`, its receipt leaving with the mailbox.
- Added the `pimdir performer` command, showing which sources can perform a capability such as `mail.submit` for the account and recording the user's choice among them (pimdir draft-03, STORAGE §15.6).
- Added `--notify`, `--ret` and `--envid` to `smtp send`, requesting delivery status notifications (RFC 3461). The command fails when the server does not announce `DSN` ([#775]).
- Added the `answered` flag to the source of `message reply --send` and the `forwarded` flag to the source of `message forward --send` once the message is sent, so it shows as replied to or forwarded in `envelope list` and in other clients. A message only printed or saved flags nothing, and a backend refusing the `$Forwarded` keyword does not fail the forward ([#778]).
- Added the source's attachments to `message forward`, carried under their file names after the quoted body. Nameless ones are named from their subject (`.eml`) or MIME type. `--no-attachments` leaves them out ([#780]).
- Added the IMAP mailbox roles: `mailbox list` shows the role a plain `LIST` marks with a special-use attribute (RFC 6154, sent unasked by Fastmail and iCloud), and a role word (`-m sent`, the trash `message delete` moves to) resolves through it, a `mailbox.alias.<role>` still winning. A server marking its roles only under `LIST RETURN (SPECIAL-USE)` still needs the aliases.
- Added `attachments` to the JSON template the composers print without `--save` or `--send`, each attached file as `{filename, mime, size}`: the template carries no bytes, so handing it back means passing the files again through `--attach`.
- Added `saved` to the receipt of `message add`, always `true`, so it has the shape of `message send`'s.

### Changed

- Changed `attachment list` and `attachment download` to print `No attachments` under the header when the message has no part to list, where the header stood alone.
- Changed `himalaya message --help` to list the subcommands by purpose, `read` and `parse`, the builders `compose`, `reply` and `forward`, the raw `send` and `add`, then `copy`, `move` and `delete`, and their help to say what each does: a builder prints the message unless `--save` or `--send`, `send` and `add` take a raw message as it is, `parse` reads a file or its contents on stdin.
- **BREAKING**: Changed `mailbox list` on Microsoft Graph to leave `total` and `unread` `null` without `--counts`, as every other backend does, where it filled them whatever the flag.
- **BREAKING**: Changed every `--json` output to data, pimalaya-cli dropping its `Message` wrapper ([pimalaya-cli every-output-is-data](https://github.com/pimalaya/cli)). Writes print what they acted on rather than a `{"message": ...}` sentence: `message copy|move` `{count, queued}`, `message send` and a composer's save or send `{id, saved, sent, queued}`, the shared receipts `queued`, a protocol write `{id}` or `{ids}`, Gmail settings writes the settings, `jmap email import` `{id, blobId}`, `jmap email read` the emails, `smtp send` the envelope, `sieve check|put` the server's `warnings`. MIGRATION.md lists every shape.
- **BREAKING**: Changed byte outputs under `--json` to `{data, size, mime}` in base64: `message read --raw` (lossy text before), `jmap email export` (failing on non-UTF-8 before), the raw Gmail and Graph `get`s and their attachment downloads (raw bytes on a JSON stdout before).
- **BREAKING**: Changed `account check` to print every backend's row, `{backend, ok, code, error}`, and exit 3 when one fails, where it exited 0 whatever failed.
- **BREAKING**: Changed every `imap` command to print its completion, `status` beside what it understood (a select's state, an expunge's numbers, a store's FETCH data, `APPENDUID`, `COPYUID`), a `NO` or `BAD` printing `{status, text}` and exiting 3. `imap raw` prints untagged and continuation responses as events, then the completions split into `tag`, `status`, `code` and `text`; `sieve raw` prints data lines as events and the completion; `smtp raw` prints `{code, text}`; each exits 3 on a refusal.
- **BREAKING**: Changed a command line Himalaya refuses (an unknown flag, a missing argument, a bad value) under `--json` to print the failure object coded `usage-invalid` on stdout, still exiting 2, where clap's text went to stderr whatever `--json` said ([pimalaya-cli a-refused-line-exits-two](https://github.com/pimalaya/cli)). A refused `repl` line and an MCP tool argument the grammar refuses (undeclared, of the wrong type or count, or a value clap refuses) carry the same code. Exit 2 is now clap's alone: a printed result that is not a full success (`account check` with a failed backend, a refused IMAP, SMTP or ManageSieve completion) exits 3.
- **BREAKING**: Changed the shapes a concept takes to one: flags written as `{raw, iana}`, `inReplyTo` a list in the read view, `imap fetch` addresses `{name, email}`, `imap search` ids numbers, attachment rows `role` instead of `inline` and `path: null`, maildir and mbox `flag list` rows `{raw, iana}`, Himalaya's own outputs printing `null` rather than omitting a key, the failure object's `code` included.
- **BREAKING**: Changed pimdir queue rows to `row` (`queueId`, `copyQueueId`, the queue view's `id`), `pimdir flag add|set|remove` to `{messages: [{id, row}]}`, and the tag statuses and search `matched` values to kebab-case.
- **BREAKING**: Changed `gmail messages list` to print `messages` rather than `ids`, and the Gmail settings lists to print the provider's key rather than `response`.
- Changed IMAP flag writes to refuse, under `capability-missing`, a keyword the mailbox's `PERMANENTFLAGS` lists neither by name nor by `\*`, which the server would keep for the session only.
- **BREAKING**: Changed the configuration loading to read every existing default path as a layer, each deep-merged on top of the ones before: $XDG_CONFIG_HOME/himalaya/config.toml, $HOME/.config/himalaya/config.toml, then $HOME/.himalayarc, which therefore overrides. Only the first one found was read, so an rc file beside a config.toml was silently ignored. A file named by two paths, a symlinked rc file being one, is read once; `-c` still replaces the default paths.
- **BREAKING**: Changed the default account resolution to refuse several accounts marked `default = true`, naming them, where one was picked in an order that changed from run to run. Naming an account with `-a` still works.
- **BREAKING**: Changed `-c/--config` to take one path per flag, never split: a value was split on `:`, which cut every absolute Windows path at its drive letter (`-c C:\Users\me\config.toml` reached the loader as `C` and `\Users\me\config.toml`). Repeat `-c` to merge several files, the first one the base and the next ones deep-merged on top.
- Changed config.sample.toml into one account listing the options every account shares with their values, then one commented section per backend or provider (IMAP and SMTP, ManageSieve, JMAP, Gmail, Microsoft Graph, Microsoft IMAP and SMTP, Fastmail, Maildir, mbox, pimdir), each a working account once uncommented, its OAuth 2.0 token read from `ortie token show`, and every variant of it parsed by the test suite. The pimdir root now names neverest's shared store, ~/.local/state/neverest, and the copied sample no longer holds an active account pointed at example.com.
- Changed the MIME parsing and composing to mime-rs (github.com/pimalaya/mime-rs, 0.1), replacing mail-parser and mail-builder: `message read`, `message parse`, `attachment list|download`, the search evaluator, the Maildir and mbox envelopes and the composers. On well-formed mail the part ids and the `--json` views stay as they were. The `text` of a `format=flowed` part, and the quote of a reply to one, is now unwrapped (RFC 3676). The plain `message read` view shows the same part types, sizes and names as the JSON view.
- Changed the composed messages to mime-rs's layout: the same headers and parts, a `Date` and `Message-ID` written by Himalaya, charset names lowercased (`utf-8''` in an RFC 2231 file name).
- Changed UTF-7 (RFC 2152) text to be shown as written: mime-rs reads the WHATWG charsets, which leave UTF-7 out.
- Changed `himalaya mcp` to list only the tools the account's transports can serve (an SMTP-only account lists `message_send`, not `envelope_list`), and to end the `envelope_search` description with the clauses and sort keys the account's backend answers fully, in part and not at all.
- Changed `message add` on Microsoft Graph, and a save asked of a composer without `--send` there, to fail before connecting with `capability-missing`, where `message add` failed with an uncoded error. A send on Graph ignores `--save`, `--no-save` and `message.send.save-copy`, Graph filing every message it sends in Sent Items itself (`submit.copy` `partial`), and its receipt says `saved: true`; it failed after sending, or failed every send when `message.send.save-copy` was set.
- Changed a JMAP send to fail with `capability-missing` before uploading anything when the server's session lacks `urn:ietf:params:jmap:submission`, rather than leave the message in the drafts mailbox.
- Changed the unified commands to be the same in every build carrying a mail transport: a build compiling SMTP alone now parses `envelope`, `flag`, `mailbox` and `attachment` and refuses them with `capability-missing`, and the shared `json-schema` output, error codes included, no longer varies with the backends compiled in.
- Changed `envelope search -h` to list every clause, operator and the sort, the grammar staying in `--help`, which now says how each backend matches text (a case-insensitive substring on IMAP and the local backends, words on Gmail, KQL on Microsoft Graph, the server's rule on JMAP) instead of promising a substring everywhere.
- Changed the shared send over SMTP to connect for each send and close after it, rather than keep the connection for the rest of the process.
- Changed `hasAttachment` on a pimdir account, and the files `pimdir attachment list` lists, to follow the store's new attachment mark (io-pimdir 925b8a1, pimdir STORAGE Annex A.1): a part with no `Content-Disposition` counts when it is neither text nor multipart and no `Content-ID` names it (a lone PDF, a forwarded message), an unknown disposition counts, and the parts of an encrypted message and the signature of a signed one never do. A message stored before keeps the mark it was stored with.
- Changed a `$Important` or `$Junk` flag change on a pimdir account to be checked against the source's `mail.flags.important` or `mail.flags.junk` when it states one, rather than `mail.flags.keywords`, so a source taking these two but no other keyword (Gmail's labels, Graph's importance) accepts them (io-pimdir 908db2b, pimdir STORAGE Annex B.1).
- Changed the confirmations of `message send`, `message add`, `message delete`, `message move`, `message copy`, `flag add|set|remove` and the composers' save and send to say the write is queued on a pimdir account (*Message queued for sending*), where the sync engine applies it later. The JSON outputs are unchanged.
- Changed `message add`, `pimdir message add` and the composers' save on a pimdir account to refuse a message whose `Message-ID` the mailbox holds or has a creation queued under, with the code `item-duplicate`, staging nothing. The row used to be queued, reported as saved, then parked by the sync engine.
- Changed the pimdir store refusals: a store of another draft is refused with `store-version`, saying to upgrade Himalaya when the store's sync engine is newer, else to reinstall the store as neverest's MIGRATION.md says, never to delete the store and let it resync, which loses the queued writes.
- Changed `message copy`, `message move` and `message delete` on a pimdir account to stage the ids the mailbox holds and report how many they found, an unknown id skipped, as on the other backends. They used to fail on the first unknown id after staging the ones before it.
- Changed `pimdir queue list` to list the mailbox's parked rows, any action kind, with why the sync engine parked each (`parked` under `--json`), and `pimdir queue cancel` to take one, naming the row's action kind (`kind` under `--json`) rather than calling every row a message.
- Bumped ctutils to 0.4.6, rustls-webpki to 0.103.16 and webpki-root-certs to 1.0.11, fixing RUSTSEC-2026-0382, RUSTSEC-2026-0383 and RUSTSEC-2026-0385.
- **BREAKING**: Changed every `--json` key to camelCase, the spelling JMAP, Microsoft Graph and the Google APIs already use, so each key is reachable by dot access in jq and JavaScript: `messages-total` becomes `messagesTotal`, `next_page` becomes `nextPage`, `thread-id` becomes `threadId`, `uid_validity` becomes `uidValidity`. Values keep their spelling (error codes, flags, mailbox roles stay kebab-case or lowercase), provider passthrough keeps the provider's keys, and the TOML configuration stays kebab-case. The `json-schema` titles follow the output types, now all named `*Output` (`Envelopes` becomes `EnvelopeListOutput`).
- Changed piped `--json` output to print one value per line, each schema of `json-schema` to carry an `$id` naming the contract version (`v2`), and added `himalaya-error`, the schema of the JSON failure object (pimalaya-cli `55b6693`).
- Changed `imap.id.auto` to send himalaya's canned `name`, `version`, `vendor` and `support-url` when `imap.id.fields` is unset, rather than `ID NIL`, which 163.com acknowledges and then refuses with `Unsafe Login`. An explicit empty `imap.id.fields` still sends `ID NIL` ([#774]).
- Changed `envelope list` and `envelope search` on a pimdir account to read only what the page needs: newest first by keyset pages up to the end of the page, a search bounded by its date clauses and reading unread mail through the store's unread index, rather than every message of the mailbox. Output is unchanged; on 50,000 mails `envelope list -s 50` goes from 3.65 s to 28 ms.
- **BREAKING**: Changed the `--json` output of `message read`, which no longer prints mail-parser's own model of the message but the view above.
- Changed the size `attachment list` reports, and the bytes `attachment download` writes, to be the part's bytes once its transfer encoding is undone, in its own charset: a text attachment is no longer converted to UTF-8, and an attached message is written alone rather than with the whole message holding it.
- Changed the pimdir writes to be refused before they are queued when a source of the store does not support them, naming the capability, the source and why, and a send to be refused while several sources can send and none is chosen. A write a source supports only in part logs a warning.
- Changed a pimdir send asking for a copy to carry it in the queued send, so the copy is filed only once the message is sent; a store whose owner declares no capabilities still gets the copy queued beside the send.
- Changed `flag add`, `flag set`, `flag remove`, `message move`, `message copy` and `message delete` (when it moves to the trash) to fail, exit code 1, when none of the given ids exists in the mailbox. Over IMAP the flag commands reported a success for such an id, and `message move` and `message copy` exited 0 on "no id matched". The flag commands now report how many messages they found, as `count` under `--json` ([#777]).
- Bumped io-pimdir to 0.11, io-msgraph to 0.7, io-jmap to 0.7, io-gmail to 0.6 and io-http to 0.6, whose fixes bound what a server can make the client hold: an HTTP response head is capped at 64 KiB and a body at 256 MiB, failing with a typed error rather than buffering without end, an IMAP greeting carrying a literal fails rather than panicking, a `STARTTLS` response line is capped, and `LOGIN` and `AUTHENTICATE` no longer log credentials at trace level. A JMAP submission refused as `tooLarge`, `forbidden`, `overQuota` or `rateLimit` now names that type rather than `unknown`. The pimdir reads take no date floor; outputs are unchanged.
- Changed the `pimdir` cargo feature to enable io-pimdir's `search` feature, which brings ical-rs and vcard-rs.
- Bumped io-imap to 0.9, io-smtp to 0.8, io-jmap to 0.8, io-gmail to 0.7, io-msgraph to 0.8, io-managesieve to 0.3, io-maildir to 0.4, io-mbox to 0.2, io-sasl to 0.2 and io-http to 0.7, all from crates.io. A PLAIN or SCRAM password is prepared with SASLprep (RFC 4013) before it is sent or derived from, a SCRAM server announcing fewer than 4096 or more than five million iterations is refused, and an IMAP command line holding a literal waits for the server's go-ahead before sending it.
- Changed `imap copy` and `imap move` to print each of `sourceUids` beside the UID it got in `destinationUids`, in the order of the `COPYUID`, where each list was sorted on its own. A `COPYUID` of over a million messages is not listed.
- Changed `smtp send` to check `--mail-from` and `--rcpt-to` as RFC 5321 mailboxes when the command line is read: a space, a control character, a line break or a second `@` is refused, a quoted local part and an address literal are taken.
- Changed the mbox backend to refuse a keyword holding a whitespace, a comma or a control character, or longer than 255 bytes, which `X-Keywords` would not give back (io-mbox).
- Changed an mbox mailbox name holding a NUL or a `.` or `..` level, or a level ending in `.lock` or `.msf`, to be refused wherever a mailbox is taken, the reads included (io-mbox).
- Changed `message copy` and `message move` on a pimdir account to fail for a message whose body is not downloaded when the account does not sync the target mailbox yet, no copy being able to reach it (io-pimdir 0c70144, pimdir SYNC §7).
- Bumped pimalaya-cli and pimalaya-config to 0.4.
- Changed the pimdir backend to follow pimdir storage-draft-11, sync-draft-11 and search-draft-11 (io-pimdir 0.12). A store or a search index an earlier neverest wrote, storage-draft-10 included, is refused, never migrated (`store-version`, `index-version`): reinstall it as neverest's MIGRATION.md says. A store whose directory or pimdir.db another user can read is refused with `store-exposed`; restrict it (`chmod 700` the directory, `chmod 600` pimdir.db). The test-only rusqlite moves to 0.41, as io-pimdir requires.
- Changed `flag add`, `flag remove` and the `\Seen` a read stages on a pimdir account to state the flags they were read from (pimdir STORAGE §15.3), so a flag a sync lands or removes while the change waits in the queue survives it, where the staged set used to overwrite it. `flag set` still replaces the whole set.
- Changed a flag change on several pimdir messages to be queued as one batch, all or none: a change one source refuses on any message stages nothing, as before, and a store failing part-way no longer leaves the first rows queued.
- Changed `pimdir queue show` to read `unconfirmed` for a send or a mailbox change the sync engine sent without learning its outcome, with what the engine recorded as `error`, where it read `parked`; to read `applied` with `collection` `null` for a row whose mailbox was deleted since, where it read `unknown`; and to know the state `reset`. `pimdir queue list` marks an unconfirmed row (`unconfirmed`, and `unconfirmed` for `parked` in the plain output), and `pimdir queue cancel` prints `unconfirmed` beside `row` and `kind`, its question and its answer saying that such a row was sent and may have taken effect.
- Changed the coverage of `pimdir search`, `count` and `tag` to carry `missing`, `unparseable` and `encrypted`, the messages in scope whose body the index could not read, could not parse or holds encrypted. They are counted whatever the query, as `bodiless` is, and leave the answer incomplete, and the `Incomplete:` line naming them, only while a term reads what they hide (pimdir SEARCH §8.2): a message whose body is not local counts against a term reading a message's body, `thread:` among them, and against `has:attachment`, so a mailbox listed and not fetched answers `is:unread` or a `date:` whole.
- Changed a `message delete` or a `message move` queued on a pimdir message a sync left in conflict to be parked by the sync engine rather than applied (pimdir SYNC §7, STORAGE §15.3), Himalaya deciding no conflict: the message stays listed where it was, and `pimdir queue list` on its mailbox and `pimdir queue show` read the row `parked` with why.
- Changed a pimdir send to name its performer in the queued `submit` whatever the sources declare, the one source syncing the mailbox when none declares capabilities; a mailbox no source syncs yet is refused with `no-performer`, staging nothing.
- Changed `pimdir flag` and `pimdir tag` to refuse a flag the store cannot hold (empty, or holding whitespace, a control or a non-ASCII character) with the code `flag-invalid`.
- Changed `pimdir search`, `count` and `tag` to refuse a query past the SEARCH §8.3 bounds (1,024 characters, 32 groups open at once, 128 terms) with `query-refused`, naming `limit-exceeded` and the span.
- Bumped rustls to 0.23.46, fixing RUSTSEC-2026-0346 to RUSTSEC-2026-0349: TLS secrets exposed by `Debug`, a downgrade to classical key exchange, and an unbounded certificate decompression.
- Changed the default of `envelope.list.datetime-fmt` to `%F %R` and of `envelope.list.datetime-local-tz` to true: a date prints as `2026-11-12 10:15` in the local timezone, where it printed `%F %R%:z` at the offset it was sent with.
- **BREAKING**: Changed every list to print as plain text, the way `mscan` prints a mailbox: one item per line in columns two spaces apart, with no header row, no border, no colour and no truncation, under the fields placing it.

  Above the items, a blank line apart, one block of fields: `Account:` names the account and the backend answering, followed by what is listed (`Mailbox:`, `Message:`, `Query:`, `Folder:`, `Thread:`); on a paged list, `Page:` names the page and what it holds (`2, envelopes 26 to 50`, `25 messages`, `, the last`, `empty`) and `Next page:` the `--page` or `--page-token` reaching the next one. `pimdir search` and `pimdir tag` print what the answer could not see as `Incomplete:`, `gmail history list` the new history id as `History id:`, `pimdir queue list` the command cancelling a row as `Cancel:`. Nothing prints under the items.

  A message prints its id, four flag columns (`*` unread, `R` answered, `!` flagged, `@` attachment, a space otherwise), its date, its senders' names (the address of a nameless one) and its subject: `envelope list|search` (or `to ` and the recipients with `--recipient`, the size no longer printing), `jmap email query|get`, `msgraph messages list`, `pimdir search` (its mailboxes and what matched before the senders, the snippet left to `--json`), and the threads `imap thread` and `pimdir thread`, a reply's subject indented two spaces per level. `pimdir tag` and `pimdir queue list` print the same shape with their status, mailbox, queue row or action.

  Every other list prints its short cells first and its free text last (`mailbox list` ends with the name), a multi-valued cell joining its values with `,`, a yes-or-no cell printing its word (`default`, `active`, `permanent`, `inline`, `subscribed`), a size with no blank (`1KiB`) and a mailbox's unread and total counts as one cell (`3/120`), so a line splits on blanks; `mailbox list` prints that cell with `--counts` only. `gmail history list` prints one line per message change. The `--json` output is unchanged.

- **BREAKING**: Changed the records to print as aligned `Label: value` lines: `account check`, `imap select`, `imap status`, `imap id`, `imap fetch`, `vacation get|set`, `jmap vacation get`, `sieve capability`, the Gmail and Microsoft Graph `get`s and the header block of `message read` and `message parse`.

  `jmap thread get` lists one email per line, and `imap search` and `imap sort` no longer end with a count.

- Changed `--help` to print without colour and each paragraph on one line, no longer wrapped to the terminal width.
- **BREAKING**: Changed every `http` or `https` URL Himalaya parses, a JMAP `server` and an HTTP proxy `url`, to take an ASCII host only, an internationalised one written in Punycode (`https://xn--bcher-kva.example` for `bücher.example`). IMAP, SMTP and ManageSieve servers never went through IDNA, a non-ASCII host there never resolving.
- Changed the subject and display names of an IMAP envelope (`envelope list`, `envelope search`, `imap fetch`, `imap thread`) to read as `message read` reads a header: a character split across two encoded words is rebuilt, raw 8-bit bytes that are not UTF-8 read as Windows-1252 rather than Latin-1, an encoded word beside them is decoded, and a UTF-7 encoded word is shown as written.
- Replaced the base64 crate with [base64-codec](https://crates.io/crates/base64-codec) in `imap fetch --body`, `attachment download --stdout`, the JMAP `Basic` header and the MCP `attach-data`, byte for byte as before, and turned on the SIMD features of the CPU, detected at startup, for it and for the base64 and quoted-printable codecs mime-rs reads messages with. An `attach-data` item whose `data` is not base64 keeps its `attachment-invalid` code, its message naming the form taken rather than the offset of the first bad byte.
- **BREAKING**: Changed `-f/--flag` of `message add`, `pimdir message add`, `imap append`, `imap store`, `maildir message save` and `mbox message save` to take one value per occurrence, so the message or the sequence set after it is never read as a flag: repeat `-f` for several flags. The message positional no longer needs `--` unless it opens on a hyphen.
- Changed `imap select` to name the first unseen message's number `firstUnseen` under `--json`, `unseen` reading as a count, and its plain output to show it, the recent count, the flags, the permanent flags and the highest mod-sequence too.
- **BREAKING**: Changed `envelope list` and `envelope search` to refuse `--page 0` and the page token `0`, which listed every envelope: `--page-size 0` lists the whole mailbox.
- **BREAKING**: Changed `--no-save` of `message compose`, `reply` and `forward` to require `--send`.
- Changed `vacation set` given none of its options to fail as a usage error before connecting, where it printed the auto-reply unchanged.
- Changed the receipts of `mailbox create` and `mailbox rename` on IMAP, Maildir and mbox to print the full name `mailbox list` prints (`Archive/2026`) rather than NAME as given, and that of `mailbox delete` the id `mailbox list` prints (a Maildir's path).
- Changed the `important` and `junk` flags on a Maildir account to be refused with `capability-missing` unless `maildir.keywords.dovecot` is on: a keyword is kept only in the dovecot-keywords file, the header mode only reading.
- **BREAKING**: Changed the `envelope search` grammar to be strict: `order by` is two words outside any pattern, so a quoted or escaped pattern holding them stays a pattern, and an empty `order by` is refused; every keyword is a whole word; a bare `"` opens a quoted pattern, an unterminated one refused, never an unquoted one (`\"` escapes it); blanks after `(` and before `)` are ignored.
- Changed the MCP tool inputs to take a lone value where the command takes one: `attachment_download`'s `attachment-id` is a string rather than an array of one, `page` and `page-size` are integers, and a repeated argument (`query`) takes a string or an array of words.
- Changed `maildir message copy` and `maildir message move`, and `mbox message copy` and `mbox message move`, to print the ids the messages got in the target rather than the source ids, each message once (io-maildir's `r#move` returning the minted id).
- Changed the root Maildir to list as `INBOX` with the inbox role, or as `.` when a top-level folder named `inbox` in any case is a Maildir and takes the role, where it listed under its folder's basename, which no command took back. `-m` takes that name, `.` or the root's path, and an omitted mailbox reaches the inbox without `mailbox.alias.inbox`; the raw `maildir` default `Inbox` falls back to the root too.
- Changed a Maildir mailbox to be taken by its name, its absolute path, with `.` segments or a trailing slash, every spelling resolving to one id; `..` and a path outside the root are refused.
- Changed a `mailbox.alias.<role>` naming no mailbox id to mark the role of the one listed mailbox carrying that name, on every backend; on Maildir an alias spelled with a trailing slash or `./` marks it too.
- **BREAKING**: Changed `maildir flag set` to require at least one `-f`, as `flag set` does: name the flags to drop with `maildir flag remove`.
- Changed `maildir create` and `maildir rename` to refuse what `mailbox create` and `rename` refuse at each level and an existing folder, `maildir rename` also the root and, on the fs layout, a target whose parent does not exist. A blank level or one holding a control character is refused too, which created folders listing under a blank or mangled name. `maildir create`, `rename` and `delete` print the name relative to the root.
- **BREAKING**: Changed `maildir message save` to refuse `-f` with `--subdir new`, its default, a Maildir keeping no flags in `new`: pass `--subdir cur`.
- Changed `message delete` on an account with neither `mailbox.alias.trash` nor a mailbox the backend marks as the trash to fail with `capability-missing`, before anything is moved.
- Changed the IMAP capability rows: `envelope.search` is `partial`, text being matched as the server matches it (RFC 3501 asks for a case-insensitive substring, Cyrus and Fastmail match whole words), which `envelope search --help` says too; `message.trash` is `server`, `mailbox.alias.trash` else the mailbox the server marks `\Trash`; `mailbox.delete` is `partial`, a mailbox holding others being refused by the servers keeping inferior names. Gmail's `envelope.search.attachment` is `partial`, its `has:attachment` counting inline images.
- Changed an IMAP account on iCloud, told by its `X-APPLE-REMOTE-LINKS` capability, to sort client-side when `imap.sort.fallback` is unset: iCloud advertises `SORT` but answers in UID order whatever the keys.
- Changed the text form of `imap raw` and `sieve raw` to print every response line as received, the untagged and data lines included, which only reached a terminal as events; the events print under `--json` only.
- Changed `message reply --send` on Gmail and Microsoft Graph, which keep no `answered` flag, to skip flagging the source rather than fail after the send.
- Changed `message send --save`, `message add --send` and a composer's `--send --save` on Gmail to label the message Gmail sent and filed under `SENT` with the mailbox and the flags, and report its id, rather than insert a second message.
- Changed `gmail messages get --format full` to print the MIME `payload` under `--json` (`null` under the other formats), and its parts with their type, file name, size and attachment id in text.
- Changed a `--backend` naming a block the account does not configure to fail with `capability-missing`.
- Changed `account check` to list the Maildir stores, so a store the listing cannot walk fails the check.

### Removed

- **BREAKING**: Removed the m2dir backend: the `himalaya m2dir` commands, the `m2dir` value of `-b/--backend`, the `m2dir` cargo feature, its JSON Schemas and the io-m2dir dependency.

  io-m2dir left the Pimalaya family, pimdir superseding it. An `m2dir` block still loads, ignored whatever it holds, and an account left with no other backend fails naming it and pointing at MIGRATION.md: move to a Maildir or a pimdir store.

- **BREAKING**: Removed the configuration wizard: bare `himalaya` running it, the `configure` command (alias `wizard`), the offer a command made without a configuration, the `wizard` cargo feature, the `himalaya-configure` JSON Schema and the io-pim-discovery dependency.

  It handed OAuth 2.0 to `ortie configure`, which Ortie 3.0 removed. Write the configuration from config.sample.toml, as the first-run welcome shows.

- **BREAKING**: Removed the `*.list.table.*-color` options, `table.preset` and `table.arrangement`.

  A colour key under `mailbox` is refused as unknown; `table`, `attachment` and `account` are ignored like any unknown top-level key.

- **BREAKING**: Removed the `envelope.list.table.unseen-char`, `replied-char`, `flagged-char` and `attachment-char` options.

  The envelope lists name the state in words (`unread`, `answered`, `flagged`, `attachment`), and the `envelope.list.table` block is ignored whatever it holds, so a v2 configuration or one shared with himalaya-tui keeps loading.

- **BREAKING**: Removed `envelope.list.datetime-relative` ([#510]), now refused as an unknown key.

  A relative date is a reader's comfort, which a frontend gives: the CLI prints a date as `envelope.list.datetime-fmt` and `datetime-local-tz` configure it.

- **BREAKING**: Removed the `HIMALAYA_CONFIG` environment variable, with the `:`-delimited list it took: write `-c "$HIMALAYA_CONFIG"` for one path, one `-c` per path for several.

- **BREAKING**: Removed `-w/--max-width` from `envelope list`, `envelope search`, `mailbox list`, `pimdir search`, `pimdir thread`, `pimdir tag`, `pimdir mailbox list` and `pimdir attachment list|search`, nothing being truncated anymore.

- Removed Himalaya's direct dependencies on open, convert_case, uds_windows, rfc2047-decoder, shellexpand and crossterm, and the toml one from the binary (the tests keep it).

  crossterm still comes with pimalaya-cli's prompt in a build with `pimdir`, and shellexpand with pimalaya-cli and pimalaya-config.

- **BREAKING**: Removed `tmp` from the `--subdir` values of `maildir message save|copy|move`: no reader lists a message there, and Maildir readers delete what stays in it for 36 hours.

### Fixed

- Fixed a message sent over SMTP, or queued on a pimdir account, taking from its headers an envelope address holding a control character, and `message send` over SMTP an address the transaction cannot carry (a space, a line break): the send fails naming the address before anything is written (io-smtp).
- Fixed `scram-sha-256` on IMAP and SMTP failing as an unsupported mechanism: the `imap` and `smtp` cargo features enable the SCRAM-SHA-256 of io-imap and io-smtp.
- Fixed an IMAP answer holding lines that do not decode being printed as whole: a warning says how many were skipped, a `message read` whose message was among them fails rather than report it missing, and a server `ALERT` is logged as a warning (io-imap).
- Fixed an mbox flag change or removal failing, or editing the wrong bytes, on an index another client made stale: the index is rebuilt and the write planned again, once (io-mbox).
- Fixed a JMAP send saying nothing when the server sends the message and refuses to move it out of the drafts: a warning says it stays a draft (io-jmap).
- Fixed a Gmail label type or setting value Himalaya does not know failing the whole answer: the text form prints it as `unknown` (io-gmail).
- Fixed IMAP commands failing with `Decode IMAP response error` when a server answers `NO` with UTF-8 in its text (Dovecot naming a non-ASCII mailbox): the refusal is printed, its 8-bit bytes read as `?` (io-imap).
- Fixed `mailbox create` and `rename` accepting a blank NAME, and the unified mbox writes a NAME holding `/`.
- Fixed `envelope search` reading options typed after the query (`... order by subject -s 2`) as query words.
- Fixed `message read` printing MIME parameters unquoted: a value that is no RFC 2045 token is quoted.
- Fixed the `message copy|move --to` help, which named no role nor id although both resolve.
- Fixed `mailbox list --counts` on pimdir leaving `unread` `null`: it is counted as `pimdir mailbox list` counts it.
- Fixed malformed multiparts reading with the wrong structure: a part an outer delimiter closes is no longer swallowed by a nested multipart (three attached messages read as one), a line opening with a boundary followed by text is no longer taken for a delimiter (the text after it was hidden), and parts mail-parser dropped (a delimiter with no blank line after it) are listed. Part ids follow the corrected structure on such mail.
- Fixed a field whose name holds a blank (`Sub ject: x`) being read as the field it spells, and a body line being read as `From:` after an unusual `Date` zone.
- Fixed `flag add|set|remove` with `answered` or `draft` on Gmail and Microsoft Graph reporting every message flagged while changing nothing: neither has a field for them, so the command fails with `capability-missing`.
- Fixed Gmail date clauses being off by a day: Gmail reads a bare date as midnight in PST, so `after 2026-09-08` kept that day's mail and `date 2026-09-08` spanned two days. Each bound is now sent as Unix seconds of the UTC day.
- Fixed a JMAP search whose date clause sits under `not` or `or` checking the date as if it stood alone: `not after 2026-01-01` listed nothing, and `date 2026-01-01 or subject x` only that day's mail. Such a clause now reads the server's received date; a date clause joined to the rest by `and` alone keeps its exact sent-date check.
- Fixed `hasAttachment` reading differently per backend ([#802]).

  IMAP missed a PDF or an attached message without a `Content-Disposition`, and Maildir and mbox counted parts marked `inline`. All three, and the local `attachment` search clause, now follow the rule of the pimdir mark: any disposition but `inline` counts; with none, a part counts unless text, multipart or named by a `Content-ID`; the signature of a signed message and the parts of an encrypted one never count. `attachment list` still lists those. An mbox cache written by an older version is rebuilt once.

- Fixed `account list` never naming `pimdir` among an account's backends, in the BACKENDS column and the `--json` `backends` alike.
- Fixed `account check` refusing an account declaring only a pimdir block as configuring no backend: the store is opened, its mailboxes listed, and a producer probed without writing anything, so a store whose staging lock a collector holds fails the check at once with `store-busy`.
- Fixed `pimdir queue cancel` reshaping the store's schema to Himalaya's io-pimdir revision, which dropped the tables and columns another app's revision keeps: the store is opened as it stands, and one of another draft is refused with `store-version`.
- Fixed `pimdir queue cancel` cancelling a row of another account or app sharing the store: only a row of the account's queue is taken.
- Fixed `pimdir queue list --json` printing `"id": ""` in a queued message's `envelope`: it prints `"id": null`, a queued message having no id until the sync engine applies it. The row id stays the row's own `id`.
- Fixed a pimdir add or send writing the message body before taking the store's staging lock, so a collector running in between could delete the body of a queued message.
- Fixed a pimdir add or send the store refuses (a duplicate, a copy the sending source does not declare) leaving its body in the store until a collector runs: the body is written only once the write is accepted.
- Fixed `body` search clauses never matching on a pimdir account, and `not body` matching every message: the local body is read. A message whose body is not local matches no body clause.
- Fixed `message read` on a pimdir message whose body file is gone failing without a code: it fails with `body-pending`, the body being not local yet.
- Fixed flag changes on a pimdir account queueing a row for a message that already shows the result.
- Fixed the plain output of `message add`, `message send`, the composers' save and send and `pimdir queue cancel` not ending with a newline.
- Fixed `flag add` and `flag remove` on a pimdir account replacing every flag a message carries on the server when the sync engine never read them: they now fail saying the flags were never synced. `flag set`, which replaces the whole set by design, is unchanged.
- Fixed `flag remove` on a pimdir account leaving a flag the store holds in another spelling than the canonical one, `\seen` or Thunderbird's `Junk`, and `flag add` adding the canonical spelling beside it: flags are now matched as flags, not as strings.
- Fixed `flag add`, `flag set` and `flag remove` on a pimdir account staging the change on the first messages of a call and then failing on a later one: every message is now checked against the sources before the first row is queued, so a call the check refuses stages nothing, and an id the mailbox does not hold is skipped and counted out, as on the other backends.
- Fixed Maildir mailboxes being named by their last path segment, so `Archive` and `[Gmail]/Archive` both listed as `Archive`: `mailbox list` and `maildir list` now name them by their path relative to `maildir.root`, the name `-m` takes, in both layouts ([himalaya-tui#22](https://github.com/pimalaya/himalaya-tui/issues/22)).
- Fixed a secret command failing silently: it now names its program and exit status instead of an empty `Secret command error` ([#745]).
- Fixed attachment downloads following dangling symlinks or overwriting existing files after filename suffixes are exhausted. Concurrent downloads now reserve filenames exclusively.
- Fixed control and bidi characters from messages and servers (bodies, headers, MIME metadata, names and ids) reaching the terminal through the plain output; the JSON and explicitly requested raw outputs are unchanged.
- Fixed pimdir queue details leaking into the unified outputs: `envelope list` no longer reports a `queued` count, and `message send`, `message add --send` and the composers no longer print a `queueId`. `himalaya pimdir queue list` shows what is queued, and a queued send logs its row id at info level.
- Fixed `message compose`, `reply` and `forward` writing a `Message-ID` on the hostname of the machine (`<…@myhost>`), which disclosed it to every recipient. The id now sits on the domain of the `From` address; without one, or with a domain that is not a plain dot-atom, the hostname is still used.
- Fixed `message move` on an IMAP server without MOVE whose `COPYUID` cannot be read, such as Mailo's UIDVALIDITY 0: the message was copied, left in the source and reported as not found. Mailo's `COPYUID` is now read despite its UIDVALIDITY 0, so a copy, or a move without MOVE, is counted without an extra round trip. Without a readable `COPYUID`, `message copy` and `message move` count the requested UIDs a `UID SEARCH` finds in the source, so a message that no longer exists is still reported as not found ([#781]).
- Fixed a raw message piped on stdin (`message send`, `message add`, `smtp send`, every `save`, `import` and `insert`) being cut at its first line that is not valid UTF-8 and still reported as sent or saved, or refused as empty when that line was the first. Stdin and message files are now read whole as bytes, so a message in another 8-bit charset goes through intact. `imap raw` and `smtp raw` read stdin whole too and refuse invalid UTF-8 rather than truncating ([#786]).
- Fixed the plain output of `flag add`, `flag set`, `flag remove` and `message delete` not ending with a newline ([#777]).
- Fixed the plain output of `maildir|mbox message save`, the Gmail draft writes, `jmap query`, `sieve get`, `sieve list` and `sieve capability` not ending with a newline, and `jmap email parse` running its bodies together.
- Fixed the query errors of `envelope search` and the pimdir search commands carrying ANSI colour escapes on a terminal, even inside the `--json` error string: they render without colour.
- Fixed a raw message positional (`message add`, `message send`, the protocol saves) naming no file being stored as a message made of that path: a value naming no file must open on a header field, and over MCP a message opening on none is refused, never read from disk.
- Fixed `message delete` in a JMAP trash destroying an email other mailboxes held too: an email the trash alone holds is destroyed, one held elsewhere only leaves the trash.
- Fixed `flag add|set|remove`, `message copy`, `message move` and `message delete` on JMAP, Gmail and Microsoft Graph acting on an id of another mailbox and counting it found, their ids being account-wide: an id counts only when its message is in the mailbox given, and a move onto its own mailbox changes nothing, where JMAP took the message out of it. On Graph, `message delete -m trash` given the id of a message in another folder deleted it for good, and a write given an unknown id among known ones stopped part-way: each id's folder is now read first, and an unknown or malformed id counts as not found.
- Fixed `message delete` in the trash reporting every id given as deleted: it counts the ids the trash held and fails when it held none.
- Fixed `flag add|set|remove`, `message copy|move|delete` given no id running on none, and a repeated id acted on and counted twice: ids are required and each counts once. The raw `maildir` and `mbox` `message copy|move` and `maildir` and `mbox` `flag add|set|remove` require their ids too.
- Fixed Maildir and mbox writes given an unknown id among known ones failing part-way or counting it: every id is looked up before the first write, an unknown one skipped, and none found fails with `no id matched`, changing nothing. The raw `maildir` `message copy|move` and `flag add|set|remove` refuse the whole set when one id is unknown, naming every unknown one, where they wrote part of it first; so do the raw `mbox` ones, which named the first only, and the raw flag commands print each id once.
- Fixed a Maildir store listing failing with `Not a directory` on a plain file beside its folders (dovecot-uidlist, dovecot-keywords), and the info flags written out of ASCII order (`:2,SF` for `:2,FS`), in io-maildir. `maildir flag list` and the `-f` values list the letters in that order too (`D`, `F`, `P`, `R`, `S`, `T`), and the `maildir` NAME help says it is the path relative to the root. A listing also skips, with a warning, a message file or a folder it cannot read and a symlink back to a folder it reached, which listed phantom mailboxes, and an experimental `:1,` info section is no longer read as flags nor rewritten as `:2,` by a flag write.
- Fixed a missing `maildir.root` listing as an empty store: it fails naming the root.
- Fixed `mailbox create` and `rename` taking an empty NAME, one holding a control character or the hierarchy delimiter, or one naming no listable level (Maildir `cur`, `new`, `tmp`, a leading dot on Maildir), and creating over or renaming onto an existing Maildir.
- Fixed `mailbox delete` on Maildir and Thunderbird-layout mbox removing the mailboxes held under the one deleted: it is refused, naming them. On IMAP a server's refusal names them too. `mailbox delete .` on Maildir emptied the whole store: the root is refused however named (`.`, `INBOX`, its path), by `mailbox rename` and the raw `maildir delete|rename` too, and a Maildir holding a folder the listing does not show (a hidden one, a half-made one) is refused, naming it.
- Fixed IMAP `mailbox create --parent` and `rename` with a non-ASCII parent or `--parent Inbox` in another case, and a missing mailbox reading as a server error: it reads `IMAP mailbox X not found`. A `--parent` the server marks `\Noinferiors` (RFC 3501 section 7.2.2) is refused before the `CREATE`.
- Fixed mbox `flag add|set|remove`, `message copy|move|delete` given ids the mailbox lacks: an unknown id is skipped, a repeated one counted once, and none found fails with `no id matched`, changing nothing.
- Fixed mbox reads carrying the `Status`, `X-Status` and `X-Keywords` fields the store writes, Thunderbird's `X-Mozilla-Status`, `X-Mozilla-Status2` and `X-Mozilla-Keys`, and the `Content-Length` of mboxcl and mboxcl2, so `message read --raw` gives back the message added.
- Fixed a Thunderbird-layout mbox rename orphaning the <name>.sbd directory of its children (it moves with the mbox, refused when the target's exists) and a delete orphaning them (refused, naming them). A flag write now keeps the read, replied, flagged and deleted bits of an existing `X-Mozilla-Status` and the keywords of an existing `X-Mozilla-Keys` in step, where a `flag set` left a dropped label for Thunderbird, a delete or rename removes the mbox's `.msf` summary, and creating `a/b` creates a missing parent `a` empty, where the child was listed under a parent no command could rename (io-mbox).
- Fixed `mailbox create --parent` under an mbox file in the plain layout, which holds no mailbox: it is refused naming `mbox.thunderbird`, and `mailbox.create` is `partial` on mbox.
- Fixed `mailbox list` on a missing `mbox.root` listing nothing: it fails naming the root.
- Fixed `mailbox delete` and `mailbox rename` deleting or renaming the mbox spool, which the mail system delivers to, from the CLI, the raw `mbox` commands and MCP alike: it is refused, as IMAP refuses it for `INBOX`, and so is a directory below the root, which is no mbox.
- Fixed `INBOX` on mbox without `mbox.inbox` silently addressing a missing file of that name below the root, and a spool that is a directory failing with an I/O error: both fail naming `mbox.inbox`, the latter in `account check` too, and a directory spool is left out of `mailbox list`.
- Fixed mbox listing files that are no mbox and appending to them: a non-empty file that does not open on a `From_` line is left out, and every message command refuses it before taking a lock, where a listing named so read as empty and a save into a symlink to a protected file failed on the dotlock (io-mbox).
- Fixed mbox mailbox names a listing hides being accepted: a level starting with `.` or ending in `.lock`, `.lck` or `.msf`, or `.sbd` with `mbox.thunderbird`, is refused, the last as the folder suffix of that layout.
- Fixed an mboxcl or mboxcl2 `Content-Length` running past the end of the file or into a later message merging the messages after it under one id: a length is trusted only when it ends on a message boundary (io-mbox).
- Fixed an mbox listing or search failing when another client rewrote the file after the index was synced: the index is rebuilt and the listing tried once more, as a read was. The cache version is raised, so older caches are rebuilt.
- Fixed a busy mbox lock failing with no code and a dotlock the user may not create failing with a bare permission error: a lock held past `mbox.lock.timeout` fails with `store-busy`, and a denied dotlock names `mbox.lock.dotlock = false`. A huge `mbox.lock.timeout` no longer panics a debug build (io-mbox).
- Fixed `message delete` on Maildir moving the messages of the trash to the trash again when the trash was named by path, with a trailing slash or through an alias: every spelling resolves to one id, so they are deleted.
- Fixed `message move` and `maildir message move` (without `--subdir`) whose source and target are one Maildir, under any spellings, re-delivering the messages under new ids: it changes nothing.
- Fixed a copy or move between Maildirs, `message delete` to the trash included, carrying dovecot keyword letters verbatim, so a keyword took the meaning the target's `dovecot-keywords` gave the same letter: with `maildir.keywords.dovecot` the letters are renamed through the target's file (io-maildir).
- Fixed a Maildir named with `-m` that cannot be searched reading as `missing cur/ subdirectory`: the permission error goes out naming the path (io-maildir).
- Fixed the local `body` search clause matching case-insensitively in ASCII only: it lowercases Unicode text, as `subject` does.
- Fixed `order by` splitting a quoted or escaped pattern holding the words, an empty `order by` being accepted, and an unterminated quote being read as a literal.
- Fixed the `envelope search` diagnostics: one report over the whole query with one `Error:`, each label quoting the word found and what was expected (`found foo, expected date after after`), no keyword fragment (`'d'`) listed, and blanks after `(` accepted. The `--help` lists one clause per line.
- Fixed the client-side sort of Maildir, mbox, pimdir, Gmail and Microsoft Graph, and IMAP's fallback (io-imap), to follow RFC 5256: `subject` on the base subject, `from` and `to` on the first address's mailbox, case ignored, ties kept in order.
- Fixed an IMAP `flag` clause naming a registered keyword in another case being searched as typed: `flag junk` sends `KEYWORD $Junk`.
- Fixed IMAP searches negating a group or an `address` clause, which iCloud answered wrongly: negations are pushed onto single keys (`not address x` sends `NOT FROM x NOT TO x NOT CC x NOT BCC x`).
- Fixed a UID Fastmail's `SORT` lists twice being listed twice.
- Fixed `message read` of an id an IMAP mailbox lacks failing without naming it: it names the id and the mailbox.
- Fixed `imap list` sending a non-ASCII pattern unencoded and its subscribed listing (`LSUB`) an unquoted one, which iCloud refuses (io-imap), and a refused `LSUB` failing rather than printing its completion.
- Fixed `imap fetch --structure` printing the server's uppercase MIME types (it lowercases them) and a bad sequence set failing without naming it, on `fetch`, `store`, `copy` and `move`.
- Fixed `imap copy` and `imap move` saying the messages were copied or moved when no `COPYUID` said so: the text names the UIDs, or says none came back.
- Fixed `imap thread` on a server without the algorithm's `THREAD=` capability failing with the server's parse error: it is refused with `capability-missing` before anything is sent.
- Fixed a JMAP write failing whole on an id gone beside known ones: an id the server answers `notFound` counts as not found, any other refusal failing the command, saying how many emails were written.
- Fixed `message add`, a save and a send on a JMAP server forbidding duplicates (Fastmail) failing on bytes the account holds: the existing email is filed into the mailbox with the flags and its id reported, or submitted.
- Fixed the JMAP set and method errors dropping the type, description and arguments a server sent (`invalidArguments [filter/inMailbox]`), and a stray backtick in their message.
- Fixed `vacation set` on Fastmail failing after the update was applied, its `newState` being `null` and its answer echoing only the properties sent (io-jmap).
- Fixed the flag receipts on JMAP printing the flag as given rather than the keyword written (`$flagged`).
- Fixed a JMAP mailbox name matching none being sent to the server, answered `invalidArguments`, and a name two mailboxes carry resolving to the first: both are refused, the second naming the ids.
- Fixed `jmap email read` printing server values unsanitized, running two emails together and exiting 0 when no id named one.
- Fixed the JMAP `query` commands returning the objects in the server's order rather than the query's (io-jmap).
- Fixed `jmap email update` dropping the patches a replacement overrode (`--keywords` with `--add-keyword`, `--mailboxes` with `--add-mailbox`) and accepting no change at all: both are refused.
- Fixed `message add -f answered` and `-f draft` on Gmail: the flags are checked against the account's capabilities, like `flag add`.
- Fixed `flag set` on Gmail failing whole by naming the `DRAFT` label, which Gmail refuses from a client.
- Fixed `mailbox rename` on Gmail leaving the sublabels under the old name: they are renamed with it, as IMAP `RENAME` moves the inferiors.
- Fixed a Gmail search pattern widening the search with the operators it held (`OR`, `AND`, braces, parentheses, `from:`): each pattern is a literal, and one holding no word is refused with `search-unsupported`.
- Fixed a Gmail reply or forward added or sent outside its thread: it carries the `threadId` of the first `In-Reply-To:` parent the account holds.
- Fixed `gmail history list` printing an added message without its labels.
- Fixed a Gmail mailbox naming no label failing with Gmail's error: it is refused naming the value.
- Fixed the Gmail snippets printing their `<b>` marks and HTML entities, and `gmail settings sendas get` printing `true` and `false` rather than `yes` and `no`.
- Fixed Gmail ids, label ids and addresses reaching the request paths unencoded (io-gmail).
- Fixed the Microsoft Graph roles dropping at random, `-m sent` then sent to Graph as a folder id: the well-known folders are looked up four to a `$batch`, as Graph serves four concurrent requests per mailbox, a throttled lookup is sent again after its `Retry-After`, and one failing other than 404 fails the command.
- Fixed `mailbox list` on Microsoft Graph leaving out the nested folders and every folder past the first page: every folder but the hidden ones is listed, a nested one named by its path (`Projets/Alpha`), which `-m` takes.
- Fixed a Microsoft Graph mailbox naming no folder being sent into the request path (`Projets/Alpha` as two segments): it is refused with `Mailbox X not found`, unless a well-known name or a folder id, then looked up. Every id reaches the path percent-encoded (io-msgraph).
- Fixed `envelope list -s 0` and `envelope search -s 0` on Microsoft Graph listing 10 messages: every page is read.
- Fixed the Microsoft Graph envelopes being dated by arrival: the date is the `Date:` header (`sentDateTime`), else the arrival.
- Fixed a Microsoft Graph search pattern holding KQL syntax (`OR`, `x:y`, `alpha*`) widening the search: each word is a quoted KQL phrase.
- Fixed `flag set` on Microsoft Graph leaving the categories it did not name: the keyword flags given replace them, none clearing them (io-msgraph).
- Fixed `msgraph messages list --search` failing on a value holding `:` or `-`, now quoted for Graph unless it opens on a double quote, and `--count` printing nothing: Graph's count prints as `count`, `Count:` in text (io-msgraph).
- Fixed an attached `message/rfc822` being written base64, which RFC 2046 forbids (mime-rs), and a forward dropping the `Content-Type` parameters (`charset`) of the source's attachments.
- Fixed a composed single-part message not ending with a line break, a quoted empty line written `> ` with a trailing blank, and the JSON template of a reply or forward carrying CRLF line breaks.
- Fixed the text view of `message read` printing `<>` for an address entry it cannot read: it prints `(unreadable: <entry>)`, as `headers.invalid` lists it.
- Fixed `attachment download` without an id writing the inline parts `attachment list` leaves out: they are written only with the new `--inline` (`-i`), as `attachment list --inline` lists them.
- Fixed `himalaya manual` leaving out the pages of the nested commands, titling a page by its last word and pointing SEE ALSO at pages it did not write (pimalaya-cli).
- Fixed the `--help` of the raw commands promising a batch of commands, which only `imap raw` takes.

## [2.2.1] - 2026-10-02

### Added

- Added `message.send.save-copy`, the mailbox a sent message is copied to when `--save` is not passed, and `--no-save` on `message send`, `compose`, `reply` and `forward` to skip it.

  It takes a mailbox name, alias or role, and the v1 `true` still reads as the `sent` mailbox. Leave it unset on Gmail and Microsoft Graph, which file the sent message themselves.

- Added `envelope.list.datetime-relative`, rendering recent dates relative to today: the time for today, `yesterday`, then the weekday for the past week, older dates falling back to `datetime-fmt` ([#510]).

- Added `proxy`, a per-account SOCKS5 or HTTP proxy every network backend connects through, and `<backend>.proxy` to override it for one backend ([#742]).

  The password is a secret like any credential, so it stays out of the URL. Without either, the `all_proxy` and `https_proxy` environment variables are still read.

- Added `--posting-style none` to `message reply` and `message forward`, sending the written body without quoting the source.

- Added a decoded template to `message compose`, `message reply` and `message forward` under `--json` when neither saving nor sending.

  It carries `from`, `to`, `cc`, `bcc`, `subject` and a `body` without the signature, so an editor can lay the message out and hand it back through the flags.

- Added `messages-added-details` to `gmail history list --json`, preserving each arrival's message id, optional thread id and supplied label ids alongside the existing message-id arrays.

- Added `message add` on Gmail, inserting the raw message through `messages.insert` with the mailbox and flags as labels, dated by its `Date:` header.

- Added `envelope search` on Microsoft Graph, its filter translated to a KQL `$search`. Flag clauses are refused, Graph accepting no `$filter` beside `$search`, and the sort applies within the page.

- A pimdir write now shows on the next read instead of on the next sync.

  A pimdir store is a replica the sync engine owns, so Himalaya appends its writes to the store's queue rather than apply them. A read used to project the committed index alone, so flagging a message lost the flag from the listing until Neverest ran.

  Reads now overlay the pending queue, so a staged flag, edit, deletion, move or copy shows straight away. All of them address a message that already exists and keeps its public id, so nothing changed about how a message is addressed.

  A staged *creation* has no id until the sync engine applies it, so it can be no envelope and does not list. `envelope list` reports how many the mailbox has and where to see them, and `envelope search` reports none, a queued message never having been matched against the query.

- `himalaya pimdir queue list` and `himalaya pimdir queue cancel`, for the staged creations the mailbox listing cannot show.

  The `pimdir` binary is kind-agnostic and prints ids, hashes and flags. Himalaya holds the mail conventions and the blobs, so it renders a queued message with its flags, subject, recipient and the moment it was queued.

  `cancel` retracts one by the row id that view prints, which is the only way back for a message that does not exist yet: a staged flag or move is undone by doing the opposite.

  Cancelling is the store owner's write, so it takes that role for the length of the call and reports a sync in flight rather than wait on it.

- Added sending from a pimdir account: the message is queued in the store for its owner to send, the one holding the credentials.

  The row is filed under the `--save` mailbox, else under `mailbox.alias.sent`, and shows in `pimdir queue list`. The command prints its row id, `queueId` under `--json`, which `pimdir queue cancel` takes and the owner reports against.

- Added ManageSieve support behind the `sieve` cargo feature, on by default ([#739]).

  An account gains an optional `[sieve]` block, and `himalaya sieve` exposes `capability`, `list`, `get`, `put`, `check`, `activate`, `deactivate`, `rename`, `delete` and `raw`.

  The protocol lives in the new [io-managesieve](https://github.com/pimalaya/io-managesieve) library, so every SASL mechanism the other backends accept works here too, and a password or a bearer token is refused over an unencrypted connection unless `sieve.allow-cleartext-auth` says otherwise.

  A bare `sieve.server` authority takes `sieve://` with STARTTLS on, where the `imap` and `smtp` ones take their implicit-TLS scheme: RFC 5804 registers one port, 4190, and reaches TLS on it through STARTTLS.

  `sieves://` is accepted for the deployments listening for a handshake straight away, and `unix://` for a local pre-authenticated proxy.

- Added the mailbox role (`inbox`, `sent`, `drafts`, `trash`...) to `mailbox list`, as a ROLE column and a `role` JSON field, with the `mailbox.list.table.role-color` option. [#743]

  JMAP reads the mailbox roles, Gmail its system labels and Graph its well-known folders, resolved in one `$batch` request. IMAP marks the inbox alone until SPECIAL-USE is read, and Maildir, m2dir and pimdir mark none. A `mailbox.alias.<role>` entry overrides the role the backend reports.

- Added `imap fetch --body`, fetching the whole message of every UID in the sequence set over one session (`BODY.PEEK[]`, `\Seen` left unset) ([#762]).

  Downloading a mailbox took one `message read` per message, so one login per message, which some providers cut off after a dozen in a row. The JSON `body` field carries the octets as standard Base64, declared as such in the JSON Schema; the plain output prints the size only.

- Added the mbox backend, behind the `mbox` cargo feature (off by default), over the new io-mbox library ([#697]).

  An `mbox` block points at a directory of mbox files and, optionally, at the spool (`$MAIL`) shown as `INBOX`. The shared commands read and write it, and `himalaya mbox` exposes the raw file operations. Any mbox file also opens by passing its absolute path as the mailbox. Writes lock the file the way MTAs and MUAs do, flags live in the `Status` and `X-Status` headers mutt and Dovecot use, and an index kept under the XDG cache directory makes a listing read only what changed.

### Changed

- Saving and sending at once (`--save` with `--send`, `message send --save`, `message add --send`) now sends first, so a failed send leaves no copy behind. A save failing after the send reports that the message was sent.

- A pimdir account sends through the store's queue and no longer uses its `smtp` section. **Behaviour change.**

- `-m/--mailbox` resolves a role after the aliases, so `-m sent` reaches the sent mailbox without an alias. A role carried by several mailboxes is an error naming them.

- An omitted `-m/--mailbox` falls back to the inbox role when no `inbox` alias is set, instead of failing.

- `message delete` takes the trash from `mailbox.alias.trash` first, then from the backend. **Behaviour change**: an alias set on JMAP, Gmail or Graph used to be ignored.

- The wizard no longer pre-fills `mailbox.alias.*`, the backends reporting their roles themselves.

- Changed Gmail draft create/update JSON output to structured draft, message and thread IDs, with schemas for both commands. Text confirmations are unchanged.

- Changed the default `--format` of `gmail messages get`, `gmail drafts get` and `gmail threads get` from `full` to `metadata`.

  None of them prints more than `metadata` returns (id, labels, snippet, headers), so their output is unchanged, and Gmail no longer sends bodies that were thrown away. Pass `--format full` to get a thread's MIME payloads.

- Forwarded `vendored` to io-pimdir and turned it on by default, so `cargo install` builds SQLite from source and needs none on the machine. Drop it to link the system SQLite and save about 1 MB; it also vendors OpenSSL when `native-tls` is on. The Nix builds still link the store's SQLite.

- **BREAKING**: a `pimdir` store written before io-pimdir 0.4 is refused, and Himalaya reads the store's typed summaries.

  io-replica is retired, its sync engine folded into io-pimdir, and the store format moved with it: summaries are typed rows rather than a JSON meta column, and a store from the earlier draft is refused on open, saying to delete it and let Neverest resync. There is no migration, the draft offering none.

  An envelope now carries every `From` and `To` address with its display name, the `In-Reply-To` list and whether the message holds an attachment, straight from the store, and `envelope list` follows the store's newest-first order. A queued creation is rendered from the body it pins, the queued action carrying no summary any more.

- A credential command that several blocks of one account name is now spawned once per run instead of once per block.

  `account check` and the wizard connection tests open IMAP, SMTP and ManageSieve in a row, each block resolving its own credential, so a `pass` or `gpg` entry was unlocked as many times as it was named. They now share one resolver, so it unlocks once.

  Two commands count as one only where the configuration wrote them identically: a shell line and the argv spelling of it stay distinct. A run reaching a single backend is unchanged.

- **BREAKING**: a `pimdir` mailbox is now its collection id, verbatim, and `pimdir.namespace` is removed.

  `-m imap/INBOX` is what `-m INBOX` used to be, and a listing shows `imap/INBOX` in both columns.

  The sync engine binds a source's collections under a namespace, and Himalaya used to strip it back off, so one mailbox answered to two spellings. The store cannot be asked where that prefix ends: a collection id is opaque to it and hierarchy is a parent link rather than a separator.

  The stripping was therefore a guess at the sync engine's convention, needing a config key wherever it could not decide. The id is now the only spelling, as it already is for JMAP's opaque ids, and `mailbox.alias.inbox = "imap/INBOX"` is how you stop typing it.

- Renamed the plural commands that name no vendor resource to their singular, the plural staying as a hidden alias.

  `imap flags`, `maildir messages`, `maildir flags`, `m2dir messages` and `m2dir flags` become `imap flag`, `maildir message`, `maildir flag`, `m2dir message` and `m2dir flag`. Every old spelling keeps working, hidden from `--help`.

  A command mirroring a vendor API resource keeps that API's spelling instead, so the `gmail` family is unchanged.

  The `msgraph` one, singular where Graph is plural, is aligned onto Graph: `mail-folder`, `message` and `attachment` become `mail-folders`, `messages` and `attachments`, joining the `child-folders` that already sat under the first.

  Every singular spelling stays as a hidden alias, where it used to be shown beside the plural, and `completion`, `manual` and `json-schema` gain the plural aliases they lacked.

- Renamed the type `configure` prints from `GeneratedConfig` to `ConfigureOutput`, the shape the rest of the family uses.

  What the command prints is unchanged. Only the `title` of the `himalaya-configure` JSON Schema follows the new name.

### Removed

- Removed the `mbox` alias of the `mailbox` command: `himalaya mbox` now runs the new mbox backend command. Scripts should call the command by its name, `himalaya mailbox`, rather than by an alias.

### Fixed

- Fixed a message sent over JMAP staying in the drafts mailbox as a draft.

  The submission now asks the server to move it to the sent mailbox, unset `$draft` and set `$seen` once sent (RFC 8621 section 7.5 `onSuccessUpdateEmail`). The sent mailbox is the `sent`-role one, or the new `jmap.sent-mailbox-id`; without either, the message only loses `$draft`.

- Fixed a successful IMAP append being reported as failed when its UID could not be recovered ([#759]).

  A server may acknowledge `APPEND` without `APPENDUID`, even with UIDPLUS, and some (QQ Exmail) also rewrite the `Message-ID` the fallback search looks for. The save now succeeds with an unknown id, and `message add --json` can return `"id": null`.

- Fixed the "No backend matching `auto`" error saying nothing of the cause ([#740]). It now names the account and either the supported backends this build compiles in or the missing `<backend>` block, and points at MIGRATION.md when the account still carries the v1 `backend` table.

- Fixed `envelope list` and `envelope search` truncating the ID column under `--max-width`.

  A long id, such as a Maildir file name, came out cut with an ellipsis and was unusable by any follow-up command. The ID column now keeps its content width, and the header, like the rows, stays on one line.

- Fixed `--to`, `--cc` and `--bcc` carrying a display name composing a `To: <Alice <alice@example.org>>` no SMTP server accepts, as `--from` did before ([#727]). Each value is now parsed as an address list, so a comma inside a quoted display name (`"Doe, Alice" <alice@example.org>`) no longer splits it.

- Fixed `imap raw` hanging until the stream timed out when the last command was not terminated by an explicit `\r\n` ([#764]).

- Fixed IMAP searches with non-ASCII text being rejected by Gmail ([io-imap#3](https://github.com/pimalaya/io-imap/issues/3)).

  Bumped io-imap to 0.7.1, whose `SEARCH` carries `CHARSET UTF-8` when its criteria hold non-ASCII text, a fix lost since v1.2.0 (#635). An ASCII search sends no charset, Outlook rejecting the UTF-8 one ([#769]).

- Fixed the `Bcc:` field being transmitted over SMTP, disclosing blind recipients to everyone (#747).

  Bumped io-smtp to 0.4, which removes it before `DATA`. The `--save` copy and `smtp send`, whose envelope is explicit, keep it.

- Fixed a local path carrying a `~` or a shell variable being read verbatim.

  `maildir.root = "~/Mail"` looked for a directory literally named `~` under the working directory rather than the Maildir the sample documents, and `m2dir.root` and the `tls.cert` of every backend read the same way.

  Every path key now expands as the configuration is read, so no reader can forget. `pimdir.root` and `downloads-dir` already expanded where they were read, and only moved onto that rule.

- Fixed a `pimdir`-only build failing to compile.

  The client-side search evaluation the backend calls was gated on the `maildir` and `m2dir` features alone, so `--no-default-features --features pimdir` did not build. The default feature set was unaffected.

- Fixed `message move` and `message delete` on IMAP servers without the MOVE extension.

  Without MOVE, a move is now `UID COPY`, `\Deleted` and, with UIDPLUS, `UID EXPUNGE` of the copied UIDs. Without UIDPLUS the messages stay flagged `\Deleted` in the source.

- Preserved message MIME payloads in `gmail threads get --json`, including full-format bodies and attachment metadata ([#750](https://github.com/pimalaya/himalaya/issues/750)).

- Fixed a duplicated message disappearing from the `pimdir` backend's listing.

  A mailbox holding one `Message-ID` twice, which a double delivery, a retried append, a restore or a copy of a sent message all produce, used to resolve to a single stored item. One copy was kept and the other recorded on it, mirrored nowhere, so it showed in no listing and could not be read.

  The store now gives the second copy a key of its own, so both list, each with its own public id and neither marked, which is what the mailbox holds. Whether they are duplicates worth removing is the server's state and the user's call, so nothing here merges or hides one.

  Saving a locally-composed message is unchanged: an add naming an id the mailbox already holds parks in the queue instead, the store being liberal with what a sync hands it and strict with what a producer creates.

- Fixed `completion` writing files instead of printing the completion script to the standard output ([#736]).

  The script has been documented as going to stdout since the command exists, but the directory it fell back on turned `himalaya completion bash` into a file-writing command printing a report.

  That broke every packaging helper capturing stdout, Homebrew's `generate_completions_from_executable` among them. Giving `--dir` keeps writing one script per shell.

  `manual` and `json-schema` follow the same shape, so the three behave alike: they print the single page or schema they are asked for, they take their directory through `--dir` where it used to be a positional, and they accept command names to select what to generate.

  Restoring the documented behaviour is a regression fix rather than a breaking change, hence the patch release, and generating completions, manual pages or schemas is not a daily gesture.

- Fixed `maildir message copy` leaving an empty message behind when the process died mid-copy ([#738]).

  The copy wrote straight to the name the target mailbox lists, so a death inside it left a 0-byte entry `envelope list` shows as an ordinary message with blank columns, `account check` calls healthy, and `message read` alone rejects.

  The copy is now staged and renamed, so an interruption leaves nothing behind.

- Fixed maildir flag writes doing nothing to a message still unread in a mailbox's `new` directory ([#637]).

  `message read --seen` and `flag add` reported success and left the message untouched, a Maildir name in `new` having nowhere to carry a flag. The message now moves to `cur` under the same id with its flags in the name, which is the transition every Maildir client performs.

## [2.2.0] - 2026-10-02

Tagged and released on GitHub, but never published to crates.io: the upload failed on `Cargo.toml` declaring six keywords, one over the limit. Its changes ship in [2.2.1].

## [2.1.0] - 2026-08-16

### Added

- Added the `pimdir` storage backend, Himalaya over the local store the sync engine populates.

  Reads build envelopes from the stored meta and report a body that is not local as "body not fetched".

  A mailbox is named the way its server names it: the sync keys a collection `<namespace>/<name>`, so the collection `imap/INBOX` is the mailbox `INBOX`, and a name matching no collection is refused naming the ones the account holds.

  The namespace is derived unless `pimdir.namespace` names it, and `pimdir.account` picks the account in a store several share.

  Himalaya reads the store read-only, which takes no lock, so a sync in flight neither blocks it nor is blocked by it.

  A write is staged as an action queued for the store's owner to apply and push, rather than written into the index: the owner's write path collects unreferenced objects, and running that beside a sync destroys the bodies it has streamed but not yet attached.

  An appended body is written to the blob store before the action referencing it, which pins it.

- Added back the `email` and `display-name` config fields, at the global and the account level ([#721]).

  They fill the `From` header of `messages compose`, `reply` and `forward` when `--from` is not passed, and are also read as `from` and `from-name`, the spellings himalaya-tui writes.

- Added back the `signature` and `signature-delim` config fields, at the global and the account level.

  The composers append the signature when neither `--signature` nor `--signature-file` is passed, `signature-delim` defaulting to the RFC 3676 §4.3 `"-- \n"` and written verbatim.

- Added `in_reply_to` to the shared envelope ([#734]).

  A list of ids stripped of their angle brackets like `message_id`, riding the JSON output only, left empty by Microsoft Graph.

- Added the shared `message delete <ids>` command (alias `del`).

  Trash-first: messages are moved to the trash mailbox, resolved from the backend role then from `mailbox.alias.trash`, or permanently removed when already in it.

- Added the `maildir.keywords.dovecot` and `maildir.keywords.header` config fields.

  Surface custom Maildir keywords as flags, read from each mailbox's dovecot-keywords file or from `X-Keywords` / `X-Label`; reading only, both off by default.

- Added the `imap.sasl-ir` config field, forcing the RFC 4959 initial response on or off ([#729]).

  Unset follows the advertised `SASL-IR` capability; `false` is needed by Coremail (126.com, 163.com), which advertises it then rejects the inline form.

- Added a `--seen` flag to `message read`, marking the message as seen while reading it.

- Added the `configure` command (alias `wizard`), running the account wizard by name.

  The first-run offer is now a hook rather than a gate: the command carries on afterwards, and nothing prompts when stdin is not a terminal or `--json` is set.

- Added back the `HIMALAYA_CONFIG` environment variable, read like `-c` and taking the same `:`-delimited list.

- Added the shared Pimalaya `--help` footer, naming the bug tracker and the sponsoring page.

### Changed

- Made the configuration wizard discovery-only, removing the hand-entry flow.

  An email, a domain, a `scheme://` URL or a local folder path all run discovery, and the wizard stops when nothing is found rather than guessing an SMTP host.

- Reworked `message read`'s plain-text output into a concise reading view.

  A header block (Date, From, To, Cc, Subject), then one summary line per MIME part with plain-text parts inlined; `--raw` and `--json` are unchanged.

- Aligned the `ID` of `message read`, `attachments list` and `attachments download` on the MIME part's 1-based position, making attachment ids a sparse subset of it.

- Right-aligned the `SIZE` column of the envelope listing table ([#723]).

- Made the wizard write the account to the configuration file rather than only print it.

  The file is where `-c` or `HIMALAYA_CONFIG` pointed, or the XDG default; an existing one gets a plain text append, so comments and formatting survive.

- Made the first-run offer open with a welcome banner on stderr, naming the configuration file it looked for; `himalaya configure` skips it.

- Changed bare `himalaya --account <NAME>` to show the help instead of running the account wizard.

- Made the wizard offer only the IMAP authentication mechanisms an unauthenticated CAPABILITY advertises, falling back to the full list on any probe failure.

- Time-bounded the wizard's service discovery, so one unreachable endpoint no longer stalls it for the operating-system connect timeout.

- Made the wizard offer to reuse the IMAP credentials for SMTP, prompting only the SMTP endpoint on accept.

- Changed `gmail messages get --header` and `gmail threads get --header` to narrow the rendered headers under every format, not only `--format metadata`.

- Changed `imap raw` to send a byte-verbatim batch of tagged commands.

  Literal `\r` / `\n` escapes decode to CRLF and the reply is read until every command is acknowledged; `smtp raw` decodes the same escapes but stays a single command.

### Fixed

- Fixed `configure --json` emitting a payload no schema described: `json-schema <DIR>` now writes himalaya-configure.json.

- Fixed `--from` carrying a display name composing a `From: <Alice <alice@example.org>>` no SMTP server accepts ([#727]).

  The value is parsed as a mailbox, the name reaching the MIME builder apart from the address.

- Fixed commands dying mid-exchange with a bare `Resource temporarily unavailable (os error 35)` ([#731], [#732]).

  The transport now retries a stream reporting it is not ready, for a minute, and arms a socket read deadline at connect time.

- Fixed lean cargo feature combinations failing to compile.

  The config schema no longer reaches for disabled backends' ALPN defaults, and every shared command is gated on the storage or send backend it needs.

- Fixed `message read --raw --json` erroring: the two now combine, emitting the raw RFC 5322 bytes as a JSON string.

- Fixed the Gmail and Microsoft Graph `get` commands wrapping their data in a `message` string under `--json` ([#730]).

  Gmail `messages get`, `drafts get`, `threads get`, `history list` and the `settings` readers now serialize their data, as does Microsoft Graph `message get`, each with a published JSON Schema.

- Fixed `gmail drafts get --format raw` printing the draft summary instead of the raw message it had fetched.

### Removed

- Removed the assets/himalaya.desktop entry and the Nix line installing it.

  Every functional line of it had stopped working, `Exec=himalaya %U` answering a mailto: link with `unrecognized subcommand`; handling mailto: is left to a wrapper of your own, as MIGRATION.md describes.

## [2.0.0] - 2026-07-26

### Added

- Added Gmail REST API support: a `[gmail]` account backend behind the shared commands and a protocol-specific `gmail` command.

  Exposes the full REST surface (profile, labels, messages, attachments, drafts, threads, history, settings), authenticating with a single OAuth 2.0 bearer token, the only authorization the API accepts.

- Added Microsoft Graph REST API support: a `[msgraph]` account backend behind the shared commands and a protocol-specific `msgraph` command.

  Exposes the Graph mail surface (profile, mail-folder, message, attachment) and mirrors `[gmail]`, authenticating with a single OAuth 2.0 bearer token.

- Restored the RFC 2971 `ID`-after-auth quirk as `imap.id.{auto, fields}`, replacing the v1.2.0 `imap.extensions.id.send-after-auth` flag.

- Brought the `m2dir` backend to CLI feature parity with `maildir`, minus mailbox `rename` and message `copy` / `move`, which await io-m2dir support.

- Added `--save <MAILBOX>` to `messages send`, mirroring the flag on `messages compose` / `reply` / `forward`.

- Added `--send` to `messages add` (alias `messages save`), mirroring `messages send --save`.

- Added the raw passthrough commands `imap raw <command>` and `smtp raw <command>`.

- Added `mailbox.alias.*` pre-filling to the wizard, so a generated account has a working `inbox` and known Sent/Drafts/Trash targets.

  JMAP reads the RFC 8621 roles live, Gmail and Microsoft Graph map their fixed system ids, and IMAP pins the reserved `INBOX` alone until io-imap can issue LIST `RETURN (SPECIAL-USE)`.

- Added the `json-schema <DIR>` command, writing one JSON Schema per structured-output command ([#547]).

  Describes each `--json` payload for downstream tooling; only compiled-in backends are emitted.

- Added a system DNS resolver fallback to discovery, making the chain `HIMALAYA_DNS_RESOLVER` then system then `1.1.1.1`.

  This keeps the email domain off a third-party resolver and works around networks blocking the default.

- Added local socket-proxy support: `imap.server` and `smtp.server` accept a `unix:///path` scheme ([#264]).

  The session arrives already authenticated from a proxy such as [sirup](https://github.com/pimalaya/sirup), so no SASL is negotiated over the socket.

### Changed

- Flattened the `imap` command tree into top-level verbs mirroring the protocol's flat command list (`select`, `create`, `append`, `store`, `fetch`), replacing the `mailbox` / `envelope` / `message` / `flag` subgroups.

- Unified raw-message input behind a single `MessageArg` (file path, inline raw or piped stdin) across every send and add command, removing the legacy `--file` flag on `messages add`.

- Split `Account` out of every client wrapper: subcommands now receive `account` and `client` as sibling arguments.

- **Breaking:** changed bare `himalaya` to run the account wizard instead of listing the default account's envelopes.

- Changed the wizard to print the generated account as a ready-to-save TOML document on stdout instead of writing it to disk.

  The connection is tested before printing, and a config that exists but lacks the requested account is now a hard error rather than a wizard trigger.

- Reworked the wizard's account-setup flow into one entry per reachable service, then a service-specific authentication prompt.

  OAuth 2.0 folds into the credential prompt, which offers the OS keyrings and the token brokers (Ortie, pizauth, oama) together.

- Changed the wizard to test IMAP first, then ask whether SMTP reuses the same credentials, then test SMTP.

- Removed the wizard's account-name prompt, the name being derived from the domain's first label or the folder name.

### Fixed

- Fixed compilation error when the `wizard` feature was disabled ([#634]).

- Fixed mailbox-alias resolution to apply across every shared `messages` subcommand and the `--save` flag on `compose` / `reply` / `forward`.

- Fixed swapped save and send success messages.

### Removed

- Removed the `[message.composer.*]` and `[message.reader.*]` config tables, with the `compose-with` / `reply-with` / `forward-with` / `mailto` / `read-with` subcommands.

  The "stdout = MIME draft" contract broke composers spawning an interactive editor; chain a standalone tool such as [mml](https://github.com/pimalaya/mml) into `messages send` / `add` instead.

- Dropped `HIMALAYA_CONFIG` environment-variable support, `-c/--config` still taking one or more `:`-separated paths.

- Removed the `account configure` command (alias `account edit`) and its per-field edit wizard, leaving `account list` and `account check` for inspection.

## [1.2.0] - 2026-02-19

### Added

- Added `--quiet` flag ([#545]).
- Added support for custom TLS certificate.
- Added support for multiple config files via `HIMALAYA_CONFIG=path1:path2:…` ([#612]).
- Added `-d|--downloads-dir` argument to `attachment download` command ([#559]).
- Added `View` and `View MIME` post edit choices ([#486]).
- Made `message send` and `template send` return id ([#627]).
- Enabled shell expansion for email, IMAP and SMTP login fields, which allows usage of environment variables ([#632]).

### Fixed

- Bumped all deps, fixing `rustls-platform-verifier` compatibility version issue.
- Fixed invalid fetch with NIL body encoding ([#525]).
- Fixed freeze when sending large attachments ([#593]).
- Fixed abusive warn logs from IMAP libs ([#552]).
- Downgraded `toml@0.8` due to regression ([#611]).

## [1.1.0] - 2025-01-11

### Added

- Added `-y|--yes` flag for `folder purge` and `folder delete` commands ([#469]).

### Changed

- Put back `warn` the default log level ([#522]).

  Since logs are sent to `stderr`, warnings can be easily discarded by prepending commands with `RUST_LOG=off` or by appending commands with `2>/dev/null`.

- Changed `message.send.save-copy` default to `true` ([#536]).

- Changed default downloads directory. [core#1]

### Fixed

- Fixed permissions issues when using `install.sh` ([#515]).
- Fixed de/serialization issues of backends' `none` variant ([#523]).
- Fixed list envelopes out of bound error when empty result ([#535]).
- Fixed macOS x86_64 builds ([#538]).

## [1.0.0] - 2024-12-09

The Himalaya CLI scope has changed. It does not include anymore the synchronization, nor the envelope watching. These scopes have moved to dedicated projects:

- [Neverest CLI](https://github.com/pimalaya/neverest), CLI to synchronize, backup and restore emails
- [Mirador CLI](https://github.com/pimalaya/mirador), CLI to watch mailbox changes

Due to the long time difference with the previous `v1.0.0-beta.4` release, this changelog may be incomplete. The simplest way to upgrade is to reconfigure Himalaya CLI from scratch, using the wizard or the [`config.sample.toml`](./config.sample.toml).

Himalaya CLI will now try to adopt the [conventional commits specification](https://github.com/conventional-commits/conventionalcommits.org). Tools like [`git-cliff`](https://git-cliff.org/) may help us generating more accurate changelogs in the future.

### Added

- Added `message edit` command to edit a message. To edit on place (replace a message), use `--on-place`.
- Added `account.list.table.preset` global config option, `accounts.<name>.folder.list.table.preset` and `accounts.<name>.envelope.list.table.preset` account config options.

  These options customize the shape of tables, see examples at [`comfy_table::presets`](https://docs.rs/comfy-table/latest/comfy_table/presets/index.html).

  Defaults to `"||  |-|||           "`, which corresponds to [`comfy_table::presets::ASCII_MARKDOWN`](https://docs.rs/comfy-table/latest/comfy_table/presets/constant.ASCII_MARKDOWN.html).

- Added `account.list.table.name-color` config option to customize the color used for the accounts' `NAME` column (defaults to `green`).
- Added `account.list.table.backends-color` config option to customize the color used for the folders' `BACKENDS` column (defaults to `blue`).
- Added `account.list.table.default-color` config option to customize the color used for the folders' `DEFAULT` column (defaults to `reset`).
- Added `accounts.<name>.folder.list.table.name-color` account config option to customize the color used for the folders' `NAME` column (defaults to `blue`).
- Added `accounts.<name>.folder.list.table.desc-color` account config option to customize the color used for the folders' `DESC` column (defaults to `green`).
- Added `accounts.<name>.envelope.list.table.id-color` account config option to customize the color used for the envelopes' `ID` column (defaults to `red`).
- Added `accounts.<name>.envelope.list.table.flags-color` account config option to customize the color used for the envelopes' `FLAGS` column (defaults to `reset`).
- Added `accounts.<name>.envelope.list.table.subject-color` account config option to customize the color used for the envelopes' `SUBJECT` column (defaults to `green`).
- Added `accounts.<name>.envelope.list.table.sender-color` account config option to customize the color used for the envelopes' `FROM` column (defaults to `blue`).
- Added `accounts.<name>.envelope.list.table.date-color` account config option to customize the color used for the envelopes' `DATE` column (defaults to `dark_yellow`).
- Added `accounts.<name>.envelope.list.table.unseen-char` account config option to customize the char used for unseen envelopes (defaults to `*`).
- Added `accounts.<name>.envelope.list.table.replied-char` account config option to customize the char used for replied envelopes (defaults to `R`).
- Added `accounts.<name>.envelope.list.table.flagged-char` account config option to customize the char used for flagged envelopes (defaults to `!`).
- Added `accounts.<name>.envelope.list.table.attachment-char` account config option to customize the char used for envelopes with at least one attachment (defaults to `@`).

### Changed

- Refactored the `account configure` command: this command stands now for creating or editing account configurations from the wizard. The command requires the `wizard` cargo feature.
- Improved the `account doctor` command: it now checks the state of the config, and the new `--fix` argument allows you to configure keyring, OAuth 2.0 etc.
- Improved long version `--version` ([#496]).
- Improved error messages when missing cargo features. For example, if a TOML configuration uses the IMAP backend without the `imap` cargo features, the error `missing "imap" feature` is displayed ([#20]).(https://github.com/pimalaya/core/issues/20)
- Normalized enum-based configurations, using the [internally tagged representation](https://serde.rs/enum-representations.html#internally-tagged) `type =`.

  It should reduce issues due to misconfiguration, and improve othe error messages. Yet it is not perfect, se ([#802]).(https://github.com/toml-rs/toml/issues/802):

  - `imap.*`, `maildir.*` and `notmuch.*` moved to `backend.*`:

	```toml
	# before
	imap.host = "localhost"
	imap.port = 143

	# after
	backend.type = "imap"
	backend.host = "localhost"
	backend.port = 143
	```

  - `smtp.*` and `sendmail.*` moved to `message.send.backend.*`:

	```toml
	# before
	smtp.host = "localhost"
	smtp.port = 25

	# after
	message.send.backend.type = "smtp"
	message.send.backend.host = "localhost"
	message.send.backend.port = 25
	```

  - `pgp.backend` renamed `pgp.type`:

	```toml
	# before
	pgp.backend = "commands"
	pgp.encrypt-cmd = "gpg --encrypt --quiet --armor <recipients>"

	# after
	pgp.type = "commands"
	pgp.encrypt-cmd = "gpg --encrypt --quiet --armor <recipients>"
	```

  - `{imap,smtp}.auth` moved as well:

    ```toml
    # before
    imap.password.cmd = "pass show example"
    smtp.oauth2.method = "xoauth2"

    # after
    backend.auth.type = "password"
    backend.auth.cmd = "pass show example"
    message.send.backend.auth.type = "oauth2"
    message.send.backend.auth.method = "xoauth2"
    ```

- Moved IMAP and SMTP `encryption` to `encryption.type`.

  This change prepares the config to accept different TLS providers with their options. The `true` and `false` variant have been removed as well:

	```toml
	# before
	backend.encryption = "none" # or false
	backend.encryption = "start-tls"
	message.send.backend.encryption = "tls" # or true

	# after
	backend.encryption.type = "none"
	backend.encryption.type = "start-tls"
	message.send.backend.encryption.type = "tls"
	```

### Fixed

- Fixed pre-release archives issue ([#492]).
- Fixed mailto parsing issue. [core#10]
- Fixed `Answered` flag not set when replying to a message ([#508]).

### Removed

- Removed systemd service from `assets/` folder, as Himalaya CLI scope does not include synchronization nor watching anymore.

## [1.0.0-beta.4] - 2024-04-16

### Added

- Added systemd service in `assets/` folder.
- Added configuration option `message.delete.style` that can be either `folder` (deleted messages are moved to the Trash folder, default style) or `flag` (deleted messages receive the Deleted flag).
- Added `--debug` as an alias for `RUST_LOG=debug`.
- Added `--trace` as an alias for `RUST_LOG=trace` and `RUST_BACKTRACE=1`.
- Added notes about `--debug` and `--trace` when error occurs.

### Changed

- **Added back the search feature**: you can now give an optional filter and sort query at the end of the `envelope list` command. See `envelope list --help` or [pimalaya.org](https://pimalaya.org/himalaya/cli/master/usage/advanced/envelope/list.html#query) for more detail on the search API.
- Changed the `envelope list` folder argument due to the search query: it became a flag `--folder|-f`.
- Made the global `--config|-c` option repeatable: the first option is considered the path to the main config, and successive options are considered partial override ([#184]).
- Refactored error management: error should be more clear, colored and can now contain spantrace and backtrace.
- Made `--help` content wrapping properly thanks to the `clap` cargo feature `wrap_help`.
- Improved `template {new,reply,forward}` command JSON output: they return now a JSON object with 3 properties:
  - `content`: the content of the template
  - `cursor.row`: the row at which the cursor should be placed by the interface using the template
  - `cursor.col`: the column at which the cursor should be placed by the interface using the template

### Fixed

- Fixed watch IMAP envelopes when folder was empt ([#179]).
- Prevented parsing of undefined config option ([#188]).
- Fixed `In-Reply-To` header being skipped from mailto URL ([#194]).
- Fixed error page out of bounds when filtering envelopes returned an empty resul ([#195]).

## [1.0.0-beta.3] - 2024-02-25

### Added

- Added `account check-up` command.
- Added wizard warning about google password ([#41]).

### Changed

- Removed account configurations flatten level in order to improve diagnostic errors, due to a [bug](https://github.com/toml-rs/toml/issues/589#issuecomment-1872345017) in clap.

  **This means that accounts need to be prefixed by `accounts`: `[my-account]` becomes `[accounts.my-account]`**. It also opens doors for interface-specific configurations.
- Rolled back cargo feature additions from the previous release. It was a mistake: the amount of features was too big, the code (both CLI and lib) was too hard to maintain.

  Cargo features kept: `imap`, `maildir`, `notmuch`, `smtp`, `sendmail`, `account-sync`, `account-discovery`, `pgp-gpg`, `pgp-commands` and `pgp-native`.
- Moved `sync.strategy` to `folder.sync.filter`.
- Changed location of the synchronization data from `$XDG_DATA_HOME/himalaya/<account-name>` to `$XDG_DATA_HOME/pimalaya/email/sync/<account-name>-cache`.
- Changed location of the synchronization cache from `sync.dir` to `$XDG_CACHE_HOME/pimalaya/email/sync/<hash>/`.
- Replaced id mapping database `SQLite` by `sled`, a pure key-val store written in Rust to improve portability of the tool. **Therefore, id aliases are reset**.
- Improved pre and post edit choices interactio ([#58]).
- Improved account synchronization performances, making it 50% faster than `mbsync` and 370% faster than `OfflineIMAP`.
- Changed `envelope.watch.{event}.{hook}`: hooks can now be cumulated. For example it is possible to send a system notification and execute a shell command when receiving a new envelope:

  ```toml
  envelope.watch.received.notify.summary = "New message from {sender}"
  envelope.watch.received.notify.body = "{subject}"
  envelope.watch.received.cmd = "echo {id} >> /tmp/new-email-counter"
  ```

### Fixed

- Fixed bug that was preventing watch placeholders to be replaced when using shell command hook.
- Fixed watch IMAP envelopes issue preventing events to be triggered.
- Fixed DNS account discovery priority issues.
- Fixed SMTP messages not properly sent to all recipient ([#172]).
- Fixed backend feature badly linked, leading to reply and forward message error ([#173]).

## [1.0.0-beta.2] - 2024-01-27

### Added

- Added cargo feature `wizard`, enabled by default.
- Added one cargo feature per backend feature:
  - `account` including `account-configure`, `account-list`, `account-sync` and the `account-subcmd`
  - `folder` including `folder-add`, `folder-list`, `folder-expunge`, `folder-purge`, `folder-delete` and the `folder-subcmd`
  - `envelope` including `envelope-list`, `envelope-watch`, `envelope-get` and the `envelope-subcmd`
  - `flag` including `flag-add`, `flag-set`, `flag-remove` and the `flag-subcmd`
  - `message` including `message-read`, `message-write`, `message-mailto`, `message-reply`, `message-forward`, `message-copy`, `message-move`, `message-delete`, `message-save`, `message-send` and the `message-subcmd`
  - `attachment` including `attachment-download` and the `attachment-subcmd`
  - `template` including `template-write`, `template-reply`, `template-forward`, `template-save`, `template-send` and the `template-subcmd`
- Added wizard capability to autodetect IMAP and SMTP configurations, based on the [Thunderbird Autoconfiguration](https://wiki.mozilla.org/Thunderbird:Autoconfiguration) standard.
- Added back Notmuch backend features.

### Changed

- Renamed `folder create` to `folder add` in order to better match types. An alias has been set up, so both `create` and `add` still work.

### Fixed

- Fixed default command: running `himalaya` without argument lists envelopes, as it used to be in previous versions.
- Fixed bug when listing envelopes with `backend = "imap"`, `sync.enable = true` and `envelope.watch.backend = "imap"` led to unwanted IMAP connection creation (which slowed down the listing).
- Fixed builds related to enabled cargo features.

## [1.0.0-beta] - 2024-01-01

Few major concepts changed:

- The concept of *Backend* and *Sender* changed. The Sender does not exist anymore (it is now a backend feature).

  A Backend is now a set of features like add folders, list envelopes or send raw message. The backend of every single feature can be customized in the configuration file, which gives users more flexibility. Here the list of backend features that can be customized:
  - `backend` ***(required)***: the backend used by default by all backend features (`maildir`, `imap` or `notmuch`)
  - `folder.add.backend`: override the backend used for creating folders (`maildir`, `imap` or `notmuch`)
  - `folder.list.backend`: override the backend used for listing folders (`maildir`, `imap` or `notmuch`)
  - `folder.expunge.backend`: override the backend used for expunging folders (`maildir`, `imap` or `notmuch`)
  - `folder.purge.backend`: override the backend used for purging folders (`maildir`, `imap` or `notmuch`)
  - `folder.delete.backend`: override the backend used for deleting folders (`maildir`, `imap` or `notmuch`)
  - `envelope.list.backend`: override the backend used for listing envelopes (`maildir`, `imap` or `notmuch`)
  - `envelope.get.backend`: override the backend used for getting envelopes (`maildir`, `imap` or `notmuch`)
  - `envelope.watch.backend`: override the backend used for watching envelopes (`maildir`, `imap` or `notmuch`)
  - `flag.add.backend`: override the backend used for adding flags (`maildir`, `imap` or `notmuch`)
  - `flag.set.backend`: override the backend used for setting flags (`maildir`, `imap` or `notmuch`)
  - `flag.remove.backend`: override the backend used for removing flags (`maildir`, `imap` or `notmuch`)
  - `message.send.backend` ***(required)***: override the backend used for sending messages (`sendmail` or `smtp`)
  - `message.read.backend`: override the backend used for reading messages (`maildir`, `imap` or `notmuch`)
  - `message.write.backend`: override the backend used for adding flags (`maildir`, `imap` or `notmuch`)
  - `message.copy.backend`: override the backend used for copying messages (`maildir`, `imap` or `notmuch`)
  - `message.move.backend`: override the backend used for moving messages (`maildir`, `imap` or `notmuch`)
- The CLI API changed: every command is now prefixed by its domain following the format `himalaya <domain> <action>`. List of domain available by running `himalaya -h` and list of actions for a domain by running `himalaya <domain> -h`.
- TOML configuration file options use now the dot notation rather than the dash notation. For example, `folder-listing-page-size` became `folder.list.page-size`. See the [changed](#changed) section below for more details.

### Added

- Added cargo feature `maildir` (not plugged yet).
- Added cargo feature `sendmail` (not plugged yet).
- Added watch hooks `envelope.watch.received` (when a new envelope is received) and `envelope.watch.any` (for any other event related to envelopes). A watch hook can be:
  - A shell command: `envelope.watch.any.cmd = "mbsync -a"`
  - A system notification: 
    - `envelope.watch.received.notify.summary = "📬 New message from {sender}"`: customize the notification summary (title)
    - `envelope.watch.received.notify.body = "{subject}"`: customize the notification body (content)

	*Available placeholders: id, subject, sender, sender.name, sender.address, recipient, recipient.name, recipient.address.*
- Added watch support for Maildir backend features.

### Changed

- Renamed cargo feature `imap-backend` → `imap`.
- Renamed cargo feature `notmuch-backend` → `notmuch`.
- Renamed cargo feature `smtp-sender` → `smtp`.
- Changed the goal of the config option `backend`: it is now the default backend used for all backend features. Valid backends: `imap`, `maildir`, `notmuch`.
- Moved `folder-aliases` config option to `folder.alias(es)`.
- Moved `folder-listing-page-size` config option to `folder.list.page-size`.
- Moved `email-listing-page-size` config option to `envelope.list.page-size`.
- Moved `email-listing-datetime-fmt` config option to `envelope.list.datetime-fmt`.
- Moved `email-listing-datetime-local-tz` config option to `envelope.list.datetime-local-tz`.
- Moved `email-reading-headers` config option to `message.read.headers`.
- Moved `email-reading-format` config option to `message.read.format`.
- Moved `email-writing-headers` config option to `message.write.headers`.
- Move `email-sending-save-copy` config option to `message.send.save-copy`.
- Move `email-hooks.pre-send` config option to `message.send.pre-hook`.
- Moved `sync` config option to `sync.enable`.
- Moved `sync-dir` config option to `sync.dir`.
- Moved `sync-folders-strategy` config option to `sync.strategy`.
- Moved `maildir-*` config options to `maildir.*`.
- Moved `imap-*` config options to `imap.*`.
- Moved `notmuch-*` config options to `notmuch.*`.
- Moved `sendmail-*` config options to `sendmail.*`.
- Moved `smtp-*` config options to `smtp.*`.
- Replaced options `imap-ssl`, `imap-starttls` and `imap-insecure` by `imap.encryption`:
  - `imap.encryption = "tls" | true`: use required encryption (SSL/TLS)
  - `imap.encryption = "start-tls"`: use opportunistic encryption (StartTLS)
  - `imap.encryption = "none" | false`: do not use any encryption
- Replaced options `smtp-ssl`, `smtp-starttls` and `smtp-insecure` by `smtp.encryption`:
  - `smtp.encryption = "tls" | true`: use required encryption (SSL/TLS)
  - `smtp.encryption = "start-tls"`: use opportunistic encryption (StartTLS)
  - `smtp.encryption = "none" | false`: do not use any encryption

### Removed

- Disabled temporarily the `notmuch` backend because it needs to be refactored using the backend features system (it should be reimplemented soon).
- Disabled temporarily the `search` and `sort` command because they need to be refactored, se ([#39]).
- Removed the `notify` command (replaced by the new `watch` command).
- Removed all global options except for `display-name`, `signature`, `signature-delim` and `downloads-dir`.

## [0.9.0] - 2023-08-28

### Added

- Added 3 new cargo features:
  - `pgp-commands`: enables the commands PGP backend (enabled by default, same behaviour as before)
  - `pgp-gpg`: enables the GPG backend (requires the `gpgme` lib on the system)
  - `pgp-native`: enables the native PGP backend
- Added account configuration `pgp` to configure the way PGP operations are performed.

### Changed

- Moved `email-writing-encrypt-cmd`to `pgp.encrypt-cmd`.
- Moved `email-reading-decrypt-cmd` to `pgp-decrypt-cmd`.
- Moved `email-writing-sign-cmd` to `pgp.sign-cmd`.
- Moved `email-reading-verify-cmd` to `pgp.verify-cmd`.

## [0.8.4] - 2023-07-18

### Fixed

- Fixed windows releases due to cargo deps typo.

## [0.8.3] - 2023-07-18

### Fixed

- Fixed windows releases due to `coredump` crate compilation error.
- Fixed macos releases due to macos 12 System Integrity Protection.

## [0.8.2] - 2023-07-18

### Changed

- Made the code async using the tokio async runtime.
- On Linux, made the kernel keyring the default one (the one based on keyutils).

### Fixed

- Fixed the way folder aliases are resolved. In some case, aliases were resolved CLI side and lib side, which led to alias error ([#95]).

## [0.8.1] - 2023-06-15

### Added

- Implemented OAuth 2.0 refresh token flow for IMAP and SMTP, which means that access tokens are now automatically refreshed and is transparent for users.
- Added `imap-oauth2-redirect-host` and `smtp-oauth2-redirect-host` options to customize the redirect server host name (default: `localhost`).
- Added `imap-oauth2-redirect-port` and `smtp-oauth2-redirect-port` options to customize the redirect server port (default: `9999`).
- Added `email-listing-datetime-fmt` to customize envelopes datetime format. See format spec [here](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).
- Added `email-listing-local-datetime` to transform envelopes datetime's timezone to the user's local one. For example, if the user's local is set to `UTC`, the envelope date `2023-06-15T09:00:00+02:00` becomes `2023-06-15T07:00:00-00:00`.

### Fixed

- Fixed missing `<` and `>` around `Message-ID` and `In-Reply-To` headers.

## [0.8.0] - 2023-06-03

### Added

- Added keyring support, which means Himalaya can now use your system's global keyring to get/set sensitive data like passwords or tokens.
- Added required IMAP option `imap-auth` and SMTP option `smtp-auth`. Possible values: `passwd`, `oauth2`.
- Added OAuth 2.0 support for IMAP and SMTP.
- Added passwords and OAuth 2.0 configuration via the wizard.
- Added `email-sending-save-copy` option to control whenever a copy of any sent email should be saved in the `sent` folder defined in `folder-aliases`.
- Imported id mapper from the lib, which means that the id mapping is now done by the CLI.
- Added `BackendConfig` to `AccountConfig::backend` to match sender implementation.
- Added support for pipeline commands, which means commands can be either a single command (string) or piped commands (list of strings). It applies for:
  - `email-writing-verify-cmd`
  - `email-writing-decrypt-cmd`
  - `email-writing-sign-cmd`
  - `email-writing-encrypt-cmd`

### Changed

- Changed release archive extensions from `.tar.gz` to `.tgz`.
- Moved `wizard` module into domains (config, account, backend…).
- [**BREAKING**] Changed the way secrets are managed. A secret is a sensitive data like passwords or tokens. There is 3 possible ways to declare a secret in the config file:
  - `{ raw = <secret> }` for the raw secret as string (unsafe, not recommended),
  - `{ cmd = <secret-cmd> }` for command that exposes the secret,
  - `{ keyring = <secret-entry> }` for entry in your system's global keyring that contains the secret.

  This applies for:
    - `imap-passwd`
	- `imap-oauth2-client-secret`
	- `imap-oauth2-access-token`
	- `imap-oauth2-refresh-token`
    - `smtp-passwd`
	- `smtp-oauth2-client-secret`
	- `smtp-oauth2-access-token`
	- `smtp-oauth2-refresh-token`

### Fixed

- Fixed Windows releases corrupted archives.

### Removed

- [**BREAKING**] Removed `-s|--sanitize` option. It is done by default now, except if the `-t|--mime-type html` is set.
- [**BREAKING**] Removed `native-tls` support, `rustls-tls` is now the only TLS provider available. Removed in consequence `native-tls`, `rustls-tls` and `rustls-native-certs` cargo features.

## [0.7.3] - 2023-05-01

### Fixed

- Fixed Windows releases (due to typo in the github action script).
- Fixed unit tests.

## [0.7.2] - 2023-05-01

### Added

- Added `create` and `delete` folder command ([#54]).
- Added generated completions and man pages to release ([#43]).
- Added new account config option `sync-folders-strategy` which allows to choose a folders synchronization strateg ([#59]).:
  
  - `sync-folders-strategy = "all"`: synchronize all existing folders for the current account
  - `sync-folders-strategy.include = ["folder1", "folder2", …]`: synchronize only the given folders for the current account
  - `sync-folders-strategy.exclude = ["folder1", "folder2", …]`: synchronizes all folders except the given ones for the current account

  Also added new `account sync` arguments that override the account config option:
  
  - `-A|--all-folders`: include all folders to the synchronization.
  - `-F|--include-folder`: include given folders to the synchronization. They can be repeated `-F folder1 folder2` or `-F folder1 -F folder2`.
  - `-x|--exclude-folder`: exclude given folders from the synchronization. They can be repeated `-x folder1 folder2` or `-x folder1 -F folder2`.

- Added cargo features `native-tls` (default), `rustls-tls` and `rustls-native-certs`.

### Changed

- Made global options truly global, which means they can be used everywhere (not only *before* commands but also *after* ([#60]).
- Replaced reply all `-a` argument with `-A` because it conflicted with the global option `-a|--account`.
- Replaced `himalaya-lib` by `pimalaya-email`.
- Renamed feature `vendored` to `native-tls-vendored`.
- Removed the `develop` branch, all the development is now done on the `master` branch.

### Fixed

- Fixed config deserialization issue with `email-hooks` and `email-reading-format`.
- Fixed flags case sensitivity.

## [0.7.1] - 2023-02-14

### Added

- Added command `folders expunge` that deletes all emails marked for deletion.
  
### Changed

- Changed the location of the [documentation](https://pimalaya.org/himalaya/).

### Fixed

- Fixed broken links in README.md.

### Removed

- Removed the `maildir-backend` cargo feature, it is now included by default.
- Removed issues section on GitHub, now issues need to be opened by sending an email at [~soywod/pimalaya@todo.sr.ht](mailto:~soywod/pimalaya@todo.sr.ht).

## [0.7.0] - 2023-02-08

### Added

- Added offline support with the `account sync` command to synchronize a backend to a local Maildir backend.
- Added the flag `--disable-cache` to not use the local Maildir backend.
- Added the email composer (from its own [repository](https://git.sr.ht/~soywod/mime-msg-builder)).
- Added Musl builds to releases.
- Added `himalaya man` command to generate man page.

### Changed

- Made commands `read`, `attachments`, `flags`, `copy`, `move`, `delete` accept multiple ids.
- Flipped arguments `ids` and `folder` for commands `copy` and `move` in order the folder not to be considered as an id.

### Fixed

- Fixed missing folder aliases.

### Removed

- Removed the `-a|--attachment` argument from `write`, `reply` and `forward` commands. Instead you can attach documents directly from the template using the syntax `<#part filename=/path/to/you/document.ext>`.
- Removed the `-e|--encrypt` flag from `write`, `reply` and `forward` commands. Instead you can encrypt and sign parts directly from the template using the syntax `<#part type=text/plain encrypt=command sign=command>Hello!<#/part>`.
- Removed the `-l|--log-level` option, use instead the `RUST_LOG` environment variable (see the [wiki](https://github.com/soywod/himalaya/wiki/Tips:debug-and-logs))

## [0.6.1] - 2022-10-12

### Added

- Added `-s|--sanitize` flag for the `read` command.
  
### Changed

- Changed the behaviour of the `-t|--mime-type` argument of the `read` command. It is less strict now: if no part is found for the given MIME type, it will fallback to the other one.

  For example, giving `-t html` will show in priority HTML parts, but if none of them are found it will show plain parts instead (and vice versa).
- Sanitization is not done by default when using the `read` command, the flag `-s|--sanitize` needs to be explicitly provided.

### Fixed

- Fixed empty text bodies when reading html part on plain text email.

## [0.6.0] - 2022-10-10

### Changed

- Separated the CLI from the lib module.

  The source code has been split into subrepositories:

  - The email logic has been extracted from the CLI and placed in a lib on [SourceHut](https://git.sr.ht/~soywod/himalaya-lib)	
  - The vim plugin is now in a dedicated repository on [SourceHut](https://git.sr.ht/~soywod/himalaya-vim) as well
  - This repository only contains the CLI source code (it was not possible to move it to SourceHut because of cross platform builds)

- [**BREAKING**] Renamed `-m|--mailbox` to `-f|--folder`

- [**BREAKING**] Refactored config system.

  The configuration has been rethought in order to be more intuitive and structured. Here are the breaking changes for the global config:

  - `name` becomes `display-name` and is not mandatory anymore
  - `signature-delimiter` becomes `signature-delim`
  - `default-page-size` has been moved to `folder-listing-page-size` and `email-listing-page-size`
  - `notify-cmd`, `notify-query` and `watch-cmds` have been removed from the global config (available in account config only)
  - `folder-aliases` has been added to the global config (previously known as `mailboxes` from the account config)
  - `email-reading-headers`, `email-reading-format`,
    `email-reading-decrypt-cmd`, `email-writing-encrypt-cmd` and
    `email-hooks` have been added
  
  The account config inherits the same breaking changes from the global config, plus:

  - `imap-*` requires `backend = "imap"`
  - `maildir-*` requires `backend = "maildir"`
  - `notmuch-*` requires `backend = "notmuch"`
  - `smtp-*` requires `sender = "smtp"`
  - `sendmail-*` requires `sender = "sendmail"`
  - `pgp-encrypt-cmd` becomes `email-writing-encrypt-cmd`
  - `pgp-decrypt-cmd` becomes `email-reading-decrypt-cmd`
  - `mailboxes` becomes `folder-aliases`
  - `hooks` becomes `email-hooks`
  - `maildir-dir` becomes `maildir-root-dir`
  - `notmuch-database-dir` becomes `notmuch-db-path`

## [0.5.10] - 2022-03-20

### Fixed

- Fixed flag commands.
- Fixed Windows build.

## [0.5.9] - 2022-03-12

### Added

- SMTP pre-send hook
- Customize headers to show at the top of a read message

### Changed

- Improve `attachments` command

### Fixed

- `In-Reply-To` not set properly when replying to a message
- `Cc` missing or invalid when replying to a message
- Notmuch backend hangs
- Maildir e2e tests
- JSON API for listings

## [0.5.8] - 2022-03-04

### Added

- Flowed format support
- List accounts command
- One cargo feature per backend

### Changed

- Vim doc about mailbox pickers

### Fixed

- Some emojis break the table layout
- Bad sender and date in reply and forward template

## [0.5.7] - 2022-03-01

### Added

- Notmuch support

### Fixed

- Build failure due to `imap` version
- No tilde expansion in `maildir-dir`
- Unknown command SORT

### Changed

- [**BREAKING**] Replace `inbox-folder`, `sent-folder` and `draft-folder` by a generic hashmap `mailboxes`
- Display short envelopes id for `maildir` and `notmuch` backends

## [0.5.6] - 2022-02-22

### Added

- Sort command
- Maildir support

### Fixed

- Suffix to downloaded attachments with same name

## [0.5.5] - 2022-02-08

### Added

- [Contributing guide](https://github.com/soywod/himalaya/blob/master/CONTRIBUTING.md)
- Notify query config option
- End-to-end encryption

### Fixed

- Multiple recipients issue
- Cannot parse address

## [0.5.4] - 2022-02-05

### Fixed

- Add attachments with save and send commands
- Invalid sequence set

## [0.5.3] - 2022-02-03

### Added

- Activate rust-imap logs when trace mode is enabled
- Set up cargo deployment

## [0.5.2] - 2022-02-02

### Fixed

- Blur in list msg screenshot
- Make inbox, sent and drafts folders customizable
- Vim plugin get focused msg id
- Nix run issue
- Range not displayed when fetch fails
- Blank lines and spaces in `text/plain` parts
- Watch command
- Mailbox telescope.nvim preview

### Removed

- The wiki git submodule

## [0.5.1] - 2021-10-24

### Added

- Disable color feature
- `--max-width|-w` argument to restrict listing table width

### Fixed

- Error when receiving notification from `notify` command

### Changed

- Remove error when empty subject
- Vim plugin does not render anymore the msg by itself, it uses the one available from the CLI

## [0.5.0] - 2021-10-10

### Added

- Mailto support
- Remove previous signature when replying/forwarding a message
- Config option `signature-delimiter` to customize the signature delimiter (default to `-- \n`)
- Expand tilde and env vars for `downloads-dir` and `signature`

### Changed

- [**BREAKING**] Folder structure, message management, JSON API and Vim plugin
- Pagination for list and search cmd starts from 1 instead of 0
- Errors management with `anyhow`

### Fixed

- Panic on flags command
- Make more use of serde
- Write message vim plugin
- Invalid encoding when sending message
- Pagination reset current account
- New/reply/forward from Vim plugin since Tpl refactor

## [0.4.0] - 2021-06-03

### Added

- Add ability to change account in with the Vim plugin
- Add possibility to make Himalaya default email app

### Changed

- [**BREAKING**] Short version of reply `--all` arg is now `-A` to
  avoid conflicts with `--attachment|-a`
- Template management

### Fixed

- `\Seen` flag when moving a message
- Attachments arg for reply and forward commands
- Vim doc

### Removed

- `Content-Type` from templates

## [0.3.2] - 2021-05-08

### Added

- Mailbox attributes
- Wiki entry about new messages counter
- Copy/move/delete a message in vim

### Changed

- Get signature from file
- [**BREAKING**] Split `idle` command into two commands:
  - `notify`: Runs `notify-cmd` when a new message arrives to the server
  - `watch`: Runs `watch-cmds` when any change occurs on the server

### Removed

- `.exe` extension from release binaries

## [0.3.1] - 2021-05-04

### Added

- Send message via stdin

### Fixed

- Table with subject containing `\r`, `\n` or `\t`
- Overflow panic when shrink column
- Vim plugin empty mailbox message

## [0.3.0] - 2021-04-28

### Fixed

- IDLE mode after network interruption
- Output redirected to `stderr`
- Refactor table system
- Editon file format on Linux
- Show email address when name not available

### Removed

- `--log-level|-l` arg (replaced by default `RUST_LOG` env var from `env_logger`)

## [0.2.7] - 2021-04-24

### Added

- Default page size to config
- Custom config path
- Setting idle-hook-cmds

### Changed

- Plain logger with `env_logger`
- Refresh email list on load buffer

### Fixed

- Improve config compatibility on Windows
- Vim table containing emoji

## [0.2.6] - 2021-04-17

### Added

- Insecure TLS option
- Completion subcommands
- Vim flags to enable telescope preview and to choose picker

### Changed

- Make `install.sh` POSIX compliant

### Fixed

- SMTP port
- Save msg upon error
- Answered flag not set
- Panic when downloads-dir does not exist
- Idle mode incorrect new message notification

## [0.2.5] - 2021-04-12

### Fixed

- Expunge mbox after `move` and `delete` cmd
- JSON output

## [0.2.4] - 2021-04-09

### Added

- Wiki entry for Gmail users
- Info logs for copy/move/delete cmd + silent mode
- `--raw` arg for `read` cmd

### Changed

- Refactor output system + log levels

## [0.2.3] - 2021-04-08

### Added

- Telescope support

### Fixed

- Unicode chars breaks the view
- Copy/move incomplete (missing parts)

## [0.2.2] - 2021-04-04

### Added

- `w` alias for `write` cmd

### Fixed

- `attachments` cmd logs
- Page size arg `search` cmd

## [0.2.1] - 2021-04-04

### Added

- IDLE support
- Improve choice after editing msg
- Flags management
- Copy feature
- Move feature
- Delete feature
- Signature support
- Add attachment(s) to a message (CLI)

### Changed

- Errors management with `error_chain`

### Fixed

- Missing `FLAGS` column in messages table
- Subtract with overflow if next page empty

## [0.2.0] - 2021-03-10

### Added

- STARTTLS support
- Flags

### Changed

- JSON support

## [0.1.0] - 2021-01-17

### Added

- Parse TOML config
- Populate Config struct from TOML
- Set up IMAP connection
- List new emails
- Set up CLI arg parser
- List mailboxes command
- Text and HTML previews
- Set up SMTP connection
- Write new email
- Write new email
- Reply, reply all and forward
- Download attachments
- Merge `Email` with `Msg`
- List command with pagination
- Icon in table when attachment is present
- Multi-account
- Password from command
- Set up README

[#264]: https://github.com/pimalaya/himalaya/issues/264
[#469]: https://github.com/pimalaya/himalaya/issues/469
[#486]: https://github.com/pimalaya/himalaya/issues/486
[#492]: https://github.com/pimalaya/himalaya/issues/492
[#496]: https://github.com/pimalaya/himalaya/issues/496
[#508]: https://github.com/pimalaya/himalaya/issues/508
[#510]: https://github.com/pimalaya/himalaya/issues/510
[#515]: https://github.com/pimalaya/himalaya/issues/515
[#518]: https://github.com/pimalaya/himalaya/issues/518
[#522]: https://github.com/pimalaya/himalaya/issues/522
[#523]: https://github.com/pimalaya/himalaya/issues/523
[#525]: https://github.com/pimalaya/himalaya/issues/525
[#535]: https://github.com/pimalaya/himalaya/issues/535
[#536]: https://github.com/pimalaya/himalaya/issues/536
[#538]: https://github.com/pimalaya/himalaya/issues/538
[#545]: https://github.com/pimalaya/himalaya/issues/545
[#547]: https://github.com/pimalaya/himalaya/issues/547
[#552]: https://github.com/pimalaya/himalaya/issues/552
[#559]: https://github.com/pimalaya/himalaya/issues/559
[#593]: https://github.com/pimalaya/himalaya/issues/593
[#611]: https://github.com/pimalaya/himalaya/issues/611
[#612]: https://github.com/pimalaya/himalaya/issues/612
[#627]: https://github.com/pimalaya/himalaya/issues/627
[#632]: https://github.com/pimalaya/himalaya/issues/632
[#634]: https://github.com/pimalaya/himalaya/issues/634
[#637]: https://github.com/pimalaya/himalaya/issues/637
[#697]: https://github.com/pimalaya/himalaya/issues/697
[#721]: https://github.com/pimalaya/himalaya/issues/721
[#723]: https://github.com/pimalaya/himalaya/issues/723
[#727]: https://github.com/pimalaya/himalaya/issues/727
[#729]: https://github.com/pimalaya/himalaya/issues/729
[#730]: https://github.com/pimalaya/himalaya/issues/730
[#731]: https://github.com/pimalaya/himalaya/issues/731
[#732]: https://github.com/pimalaya/himalaya/issues/732
[#734]: https://github.com/pimalaya/himalaya/issues/734
[#736]: https://github.com/pimalaya/himalaya/issues/736
[#738]: https://github.com/pimalaya/himalaya/issues/738
[#739]: https://github.com/pimalaya/himalaya/issues/739
[#740]: https://github.com/pimalaya/himalaya/issues/740
[#742]: https://github.com/pimalaya/himalaya/issues/742
[#743]: https://github.com/pimalaya/himalaya/issues/743
[#745]: https://github.com/pimalaya/himalaya/issues/745
[#759]: https://github.com/pimalaya/himalaya/issues/759
[#762]: https://github.com/pimalaya/himalaya/issues/762
[#764]: https://github.com/pimalaya/himalaya/issues/764
[#769]: https://github.com/pimalaya/himalaya/issues/769
[#772]: https://github.com/pimalaya/himalaya/issues/772
[#774]: https://github.com/pimalaya/himalaya/pull/774
[#775]: https://github.com/pimalaya/himalaya/issues/775
[#777]: https://github.com/pimalaya/himalaya/issues/777
[#778]: https://github.com/pimalaya/himalaya/issues/778
[#779]: https://github.com/pimalaya/himalaya/issues/779
[#780]: https://github.com/pimalaya/himalaya/issues/780
[#787]: https://github.com/pimalaya/himalaya/issues/787
[#781]: https://github.com/pimalaya/himalaya/issues/781
[#786]: https://github.com/pimalaya/himalaya/issues/786
[#790]: https://github.com/pimalaya/himalaya/issues/790
[#802]: https://github.com/pimalaya/himalaya/issues/802
[#805]: https://github.com/pimalaya/himalaya/issues/805
[#806]: https://github.com/pimalaya/himalaya/issues/806

[core#1]: https://github.com/pimalaya/core/issues/1
[core#10]: https://github.com/pimalaya/core/issues/10

[unreleased]: https://github.com/pimalaya/himalaya/compare/v2.2.1...HEAD
[2.2.1]: https://github.com/pimalaya/himalaya/compare/v2.1.0...v2.2.1
[2.2.0]: https://github.com/pimalaya/himalaya/compare/v2.1.0...v2.2.0
[2.1.0]: https://github.com/pimalaya/himalaya/compare/v2.0.0...v2.1.0
[2.0.0]: https://github.com/pimalaya/himalaya/compare/v1.2.0...v2.0.0
[1.2.0]: https://github.com/pimalaya/himalaya/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/pimalaya/himalaya/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/pimalaya/himalaya/compare/v1.0.0-beta.4...v1.0.0
[1.0.0-beta.4]: https://github.com/pimalaya/himalaya/compare/v1.0.0-beta.3...v1.0.0-beta.4
[1.0.0-beta.3]: https://github.com/pimalaya/himalaya/compare/v1.0.0-beta.2...v1.0.0-beta.3
[1.0.0-beta.2]: https://github.com/pimalaya/himalaya/compare/v1.0.0-beta...v1.0.0-beta.2
[1.0.0-beta]: https://github.com/pimalaya/himalaya/compare/v0.9.0...v1.0.0-beta
[0.9.0]: https://github.com/pimalaya/himalaya/compare/v0.8.4...v0.9.0
[0.8.4]: https://github.com/pimalaya/himalaya/compare/v0.8.3...v0.8.4
[0.8.3]: https://github.com/pimalaya/himalaya/compare/v0.8.2...v0.8.3
[0.8.2]: https://github.com/pimalaya/himalaya/compare/v0.8.1...v0.8.2
[0.8.1]: https://github.com/pimalaya/himalaya/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/pimalaya/himalaya/compare/v0.7.3...v0.8.0
[0.7.3]: https://github.com/pimalaya/himalaya/compare/v0.7.2...v0.7.3
[0.7.2]: https://github.com/pimalaya/himalaya/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/pimalaya/himalaya/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/pimalaya/himalaya/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/pimalaya/himalaya/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/pimalaya/himalaya/compare/v0.5.10...v0.6.0
[0.5.10]: https://github.com/pimalaya/himalaya/compare/v0.5.9...v0.5.10
[0.5.9]: https://github.com/pimalaya/himalaya/compare/v0.5.8...v0.5.9
[0.5.8]: https://github.com/pimalaya/himalaya/compare/v0.5.7...v0.5.8
[0.5.7]: https://github.com/pimalaya/himalaya/compare/v0.5.6...v0.5.7
[0.5.6]: https://github.com/pimalaya/himalaya/compare/v0.5.5...v0.5.6
[0.5.5]: https://github.com/pimalaya/himalaya/compare/v0.5.4...v0.5.5
[0.5.4]: https://github.com/pimalaya/himalaya/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/pimalaya/himalaya/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/pimalaya/himalaya/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/pimalaya/himalaya/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/pimalaya/himalaya/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/pimalaya/himalaya/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/pimalaya/himalaya/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/pimalaya/himalaya/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/pimalaya/himalaya/compare/v0.2.7...v0.3.0
[0.2.7]: https://github.com/pimalaya/himalaya/compare/v0.2.6...v0.2.7
[0.2.6]: https://github.com/pimalaya/himalaya/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/pimalaya/himalaya/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/pimalaya/himalaya/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/pimalaya/himalaya/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/pimalaya/himalaya/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/pimalaya/himalaya/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/pimalaya/himalaya/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/pimalaya/himalaya/releases/tag/v0.1.0

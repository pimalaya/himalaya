# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [3.0.0] - 2026-11-25

Himalaya 3 is a low-level CLI over a unified mail API: the same commands on every backend, the protocol-specific ones beside them, plain text for a glance and `--json` for scripts, frontends and AI agents. Daily-mail comfort (tables, colours, relative dates, rich composition) moves to frontends such as himalaya-tui and to mml, and the offline side to the pimdir store Neverest syncs, which Himalaya reads and queues its writes into.

The wizard, the m2dir backend and the display options are gone, an unknown configuration key fails the load, and every failure carries a stable `code`. MIGRATION.md lists what to act on when upgrading from 2.x.

### Added

- Added the account-level `tls` table (`tls.provider`, `tls.rustls.crypto`, `tls.cert`, `tls.client.cert`, `tls.client.key`), inherited field by field by every network backend.

  A backend's own `tls` wins, so one trusted certificate reaches IMAP, SMTP, ManageSieve, JMAP, Gmail and Microsoft Graph alike.

- Added `tls.client.cert` and `tls.client.key` to every network backend, a client certificate presented for mutual TLS.

  native-tls reads a PKCS#8 key only.

- Added `maildir.layout`, `fs` by default or `maildir++` for a Dovecot or Courier store.

  The root is the inbox, `.a.b` lists as `a/b`, and a level holding a dot is refused.

- Added `message.reply.posting-style` and `message.forward.posting-style`, global or per account, the style used without `--posting-style` ([#772]).
- Added the first-run welcome: at a terminal without a configuration, a command prints the missing path and the commands copying config.sample.toml in place, then exits 1.

  Under `--json` or with stderr piped, the error names the path and the sample instead.

- Added config.sample.toml to the Nix package and the release archives as share/doc/himalaya/config.sample.toml, which install.sh installs under its prefix.
- Added a stable `code` to the JSON failure object for the failures a caller is expected to act on, `null` otherwise.

  `json-schema himalaya-error` describes the object and lists every code.

- Added the item and mailbox codes `message-not-found`, `mailbox-not-found`, `message-unreadable`, `mailbox-unreadable`, `mailbox-duplicate`, `message-too-complex`, `thread-not-found`, `identity-not-found` and `submission-not-found`.
- Added the server refusal codes `permission-denied`, `quota-exceeded`, `rate-limited` (a Google daily quota included) and `recipient-refused`.
- Added the usage codes `usage-invalid`, `capability-missing`, `search-invalid`, `search-unsupported`, `page-token-invalid`, `stdin-taken`, `attachment-too-large`, `attachment-invalid` and `request-origin`.
- Added the account codes `account-missing`, `account-default-missing`, `account-default-several` and `backend-not-built`, the last naming the cargo feature to rebuild with.
- Added the pimdir codes io-pimdir gives a store, its queue and its search index.

  Store: `store-version`, `store-uncreated`, `store-foreign`, `store-corrupt`, `store-not-current`, `store-hash-algo`, `store-busy`, `store-owned`, `store-staging`, `store-in-flight`, `store-exposed`, `store-reset`, `source-empty`, `store-account-ambiguous`, `store-account-unknown`, `sync-version`, `lock-replaced`.

  Queue and index: `body-pending`, `item-duplicate`, `flag-invalid`, `action-malformed`, `conflict-unread`, `unsupported`, `no-performer`, `ambiguous-performer`, `row-not-found`, `confirmation-required`, `index-missing`, `index-outdated`, `index-version`, `index-zone-unknown`, `index-fts5-missing`, `query-refused`, `query-unanswerable`, `tag-refused`.

- Added `errors` to `envelope list|search`, `mailbox list`, `flag add|set|remove` and `message copy|move|delete`, one item error `{id, code, error, sources}` per item the command could not list or act on.

  A non-empty `errors` exits 3. In text each entry prints as an `Error:` line after the items.

- Added a JSON Schema for every command, `himalaya-error` for the failure object, and `.events` schemas for what `imap raw` and `sieve raw` print before their result.
- Added `total` to `envelope list --json`, how many messages the mailbox holds, `null` on Gmail and Microsoft Graph and for `envelope search`.
- Added `--page-token` to `envelope list|search` and `jmap email|mailbox|submission query`, taking back the `nextPage` a page printed, `null` on the last page.

  It is refused beside `--page`, and a token no listing printed fails `page-token-invalid`.

- Added `--page-token` to `msgraph messages list`, `mail-folders list` and `mail-folders child-folders`, following as is the Graph next link they print as `nextPage`.

  A query option beside it is refused (exit 2), the link carrying the query.

- Added a library target, which himalaya-tui pins by git revision to run Himalaya's commands in-process, with no semver promise beyond the CLI grammar and `--json`.
- Added `himalaya repl`, a session running one command per line over clients connected once for the session.

  Piped, every line gets exactly one answer; a line setting `-a`, `-c`, `--log-level` or `-b`, or reading stdin (`stdin-taken`), is refused.

- Added `himalaya mcp`, a Model Context Protocol server over stdio exposing the account's unified API as tools named after their command.

  Only the reading tools are listed by default: `--allow-write` adds the writes and `--allow-send` the sends, which keep no copy without `--allow-write`.

- Added the MCP tool `attachment_download` and the `attach-data` argument of `message_compose`, `message_reply` and `message_forward` ([#805], [#806]).

  Both refuse a part over `himalaya mcp --max-attachment-size`, 10 MiB by default, coded `attachment-too-large`.

- Added `account capability`, listing what the account serves of the unified API: per capability its backend, its level (`full`, `partial`, `server`, `none`) and the detail.
- Added a capability table per backend: a unified command reaching a capability its backend declares `none` is refused before connecting, coded `capability-missing`.

  The `--help` of every unified command ends with a generated `Backends:` footer listing those levels.

- Added `mailbox create`, `mailbox rename` and `mailbox delete` to the unified API, on every backend.

  A delete takes the messages the mailbox holds; on pimdir each is queued for the sync engine.

- Added the `important` and `junk` flags (RFC 8457 `$Important`, `$Junk`) to `flag add|set|remove`.

  Graph refuses `junk`, Gmail's `junk` is the `SPAM` label, and Maildir needs `maildir.keywords.dovecot`.

- Added `vacation get` and `vacation set`, reading and changing the auto-reply on JMAP and Gmail, whatever configured backend holds the mail.

  IMAP refuses it, naming `himalaya sieve`.

- Added `message parse [EML]`, printing the `message read` view of a raw message from a file or stdin with no account, `--part` and `--section` writing one part.
- Added `--stdout` to `attachment download`, writing one part's decoded bytes to stdout, or `{id, mime, filename, size, data}` with `data` in base64 under `--json`.
- Added `contentId` to `attachment list --json`.
- Added `--all` (`-A`) to `message reply`, replying to everyone but the account's own address ([#779]).
- Added the source's attachments to `message forward`, `--no-attachments` leaving them out ([#780]).
- Added the `answered` flag to the source of `message reply --send` and the `forwarded` flag to that of `message forward --send` ([#778]).

  Gmail and Microsoft Graph keep no `answered` flag, so a reply there flags nothing.

- Added `date`, `messageId` and `attachments` to the JSON template `message compose|reply|forward` print without `--save` or `--send`.

  `attachments` lists `{filename, mime, size}` with no bytes: hand the files back through `--attach`.

- Added `--notify`, `--ret` and `--envid` to `smtp send`, requesting delivery status notifications (RFC 3461) ([#775]).

  The command fails when the server does not announce `DSN`.

- Added `from`, `date` and `flags` to each `imap thread` entry.
- Added the IMAP mailbox roles a plain `LIST` marks with a special-use attribute (RFC 6154), shown by `mailbox list` and taken by `-m`.

  A `mailbox.alias.<role>` still wins.

- Added `jmap request`, sending one whole JMAP request object and printing `{response}`, as Cardamum's does.

  `jmap query`, taking a method-calls array, stays as a hidden command.

- Added `gmail request` and `msgraph request`, sending one request to the Gmail or Microsoft Graph API and printing `{response}`, the provider's answer.

  A URL off the API's origin is refused before the session opens, coded `request-origin`.

- Added `msgraph messages send-draft`, sending an existing Microsoft Graph draft ([#787]).
- Added `--no-categories` to `msgraph messages update`, removing every category of the message.
- Added `pimdir search`, searching the account's mail in the pimdir query language (SEARCH.md §8) from the store's search index.

  `--sort`, `--threads` and `--page-token` shape the answer; each hit carries its envelope, mailboxes and a snippet, and `coverage` says what the index could not see.

- Added `pimdir count`, counting the account's mail a query matches, with its coverage.
- Added `pimdir thread`, listing the thread of a message from the search index, oldest first.
- Added `pimdir tag`, queueing a flag change on every message a query matches, `--dry-run` queueing nothing.
- Added `pimdir flag list|add|set|remove`, reading and staging any flag of a pimdir message by its raw spelling, keywords included ([#790]).
- Added `pimdir mailbox list`, listing each mailbox with its `total` and `unread` counts and the `coverage` the sync engine last listed it within.
- Added `pimdir mailbox create`, `rename` and `delete`, queueing the change for the sync engine to apply on the server.

  A delete takes the messages; a mailbox holding other mailboxes parks, naming them.

- Added `pimdir message add` and `pimdir message send`, staging a message and printing its link id as `id` with its queue row.
- Added `pimdir queue show`, printing a row as `pimdir queue list` does, then its `state` (`pending`, `parked`, `unconfirmed`, `applied`), `appliedAt` and `refusals`.
- Added `pimdir attachment list` and `pimdir attachment search`, the files the account's messages attach, newest first, each with the `part` `attachment download` takes.
- Added `pimdir performer`, showing which sources can perform an intent (`mail.submit`, `collection.create`, `collection.rename`, `collection.delete`) and recording the user's choice.
- Added `pimdir status`, reporting per mailbox how many messages carry a local body and how many writes are queued, with the parked rows and the standing refusals.
- Added the role of a pimdir mailbox, recorded by the sync engine: `mailbox list` shows it, and a role name addresses the mailbox wherever one is taken.

### Changed

- **BREAKING**: Changed the top level, the account blocks and the `message` table to refuse an unknown key, naming it and its line, as every other table did.

  An `m2dir` block, a v1 `backend` or `folder` table and the rest of a v1 `[message]` table are refused with them; himalaya-tui's keys go under `[tui]` and `[accounts.<name>.tui]`.

- **BREAKING**: Changed the configuration loading to read every existing default path as a layer, each deep-merged on top of the ones before.

  $XDG_CONFIG_HOME/himalaya/config.toml, then $HOME/.config/himalaya/config.toml, then $HOME/.himalayarc, which therefore overrides; `-c` still replaces them.

- **BREAKING**: Changed `-c/--config` to take one path per flag, never split on `:`: repeat `-c` to merge several files.
- **BREAKING**: Changed the default account resolution to refuse several accounts marked `default = true`, naming them.
- **BREAKING**: Changed `-a default` to name an account called `default`, where it took the account marked default.
- **BREAKING**: Changed `pimdir.account` left unset to be derived from the accounts holding mail, several being refused coded `store-account-ambiguous`.

  A `pimdir.account` a store holding collections does not hold is refused coded `store-account-unknown`, where it read as an empty account.

- **BREAKING**: Changed every `http` or `https` URL, a JMAP `server` and an HTTP proxy `url`, to take an ASCII host only: write an internationalised one in Punycode.
- Changed the default of `envelope.list.datetime-fmt` to `%F %R` and of `envelope.list.datetime-local-tz` to true, printing dates in the local timezone.
- Changed config.sample.toml into one account listing every key of every backend, commented at its default or an example value.
- Changed `imap.id.auto` to send Himalaya's `name`, `version`, `vendor` and `support-url` when `imap.id.fields` is unset, rather than `ID NIL` ([#774]).

  An explicit empty `imap.id.fields` still sends `ID NIL`.

- Changed the Nix package to build from the repository's package.nix.
- **BREAKING**: Changed every `--json` key to camelCase (`messagesTotal`, `nextPage`, `threadId`, `uidValidity`), values and provider passthrough keeping their spelling.

  The TOML configuration stays kebab-case, and the `json-schema` titles are named `*Output` (`EnvelopeListOutput`).

- **BREAKING**: Changed every `--json` output to data: a write prints what it acted on rather than a `{"message": ...}` sentence.

  `message copy|move` print `{count, queued}`, a send or a save `{id, saved, sent, queued, errors}`, `message delete` `{action, count, ids, queued, errors}` and a protocol write `{id}` or `{ids}`. A send only queued on pimdir prints `sent: false`; MIGRATION.md lists every shape.

- **BREAKING**: Changed byte outputs under `--json` to `{data, size, mime}` in base64: `message read --raw`, `jmap email export` and the raw Gmail and Graph `get`s and attachment downloads.
- **BREAKING**: Changed the shapes a concept takes to one: flags `{raw, iana}`, `imap fetch` addresses `{name, email}`, `imap search` ids numbers, attachment rows `role` rather than `inline`.

  Himalaya's own outputs print `null` rather than omit a key.

- **BREAKING**: Changed `subject` to `null` for a message without one in every envelope and `imap thread` entry, where it printed `""`.
- **BREAKING**: Changed an envelope's `size` to `null` when the backend gives none, where it printed `0`: always on Microsoft Graph, sometimes on JMAP and pimdir.
- **BREAKING**: Changed a UTC instant to end with `Z` in `imap fetch --internal-date` and the `message read` view, where it ended with `+00:00`.
- **BREAKING**: Changed `message read --json` to print the envelope row and a view the same on every backend, no longer mail-parser's model of the message.

  The view holds `headers` (addresses as `{name, email}`, the unreadable ones under `invalid`), the decoded `text` and `html`, and `parts`. A message nested deeper than 8 levels, or holding more than 200 parts or 500 header lines, fails `message-too-complex`.

- Changed piped `--json` output to print one value per line, and each `json-schema` schema to carry an `$id` naming the contract version.
- **BREAKING**: Changed a command line Himalaya refuses under `--json` to print the failure object coded `usage-invalid` on stdout, still exiting 2.

  Exit 2 is now clap's alone: a printed result that is not a full success exits 3.

- **BREAKING**: Changed a bare `himalaya --json` to fail coded `usage-invalid`, exit 2, where it printed the help.
- **BREAKING**: Changed a server's refusal in the `gmail`, `msgraph`, `jmap`, `sieve` and `smtp` namespaces to print as the command's result and exit 3, where it printed the failure object and exited 1.

  An HTTP or JMAP refusal prints `{refusal: {status, reason, message, code, retryAfter}}`, a `sieve` `NO` `{status, code, text}`, a refused `smtp send` `{refusals: [{rcptTo, code, text}]}`.

- **BREAKING**: Changed every `imap` command to print its completion, `status` beside what it understood, a `NO` or `BAD` printing `{status, text}` and exiting 3.

  `imap raw` and `sieve raw` print untagged and data responses as events, then the completion, and `smtp raw` prints `{code, text}`; their text form prints every line as received.

- **BREAKING**: Changed `imap raw` to print each response from its exact octets, one that is not UTF-8 printing as an `undecoded` event `{base64}`, where its 8-bit bytes became U+FFFD.
- **BREAKING**: Changed the values a protocol command refuses itself to fail with the command line, coded `usage-invalid`, exit 2, before the password command runs.

  An `imap fetch|store|copy|move` sequence set, `jmap submission create` given one side of the envelope, `msgraph messages update` naming no change, `smtp send --notify never` beside another condition, and an `imap raw` command without a tag of its own.

- **BREAKING**: Changed the `gmail`, `msgraph` and `jmap` commands to print the provider's payload as it sent it, every property kept, where they printed typed rows dropping what the type did not model.

  `gmail profile get` prints `emailAddress`, `gmail messages list` `messages` rather than `ids`, `gmail history list` Gmail's records rather than `messages-added-details`, the Gmail settings lists their own key rather than `response`, `msgraph messages list --count` Graph's `@odata.count`.

- **BREAKING**: Changed `gmail labels get` and `msgraph mail-folders get` to take one id and print that label or folder, where they took several and printed a list.
- **BREAKING**: Changed `jmap email delete|update|copy`, `jmap mailbox destroy`, `jmap identity delete` and `jmap submission cancel` to print `{ids, errors}` and exit 3 when the server refused part of the set, where they exited 1.
- **BREAKING**: Changed `jmap email get|query`, `jmap mailbox get|query`, `jmap thread get`, `jmap identity get` and `jmap submission get|query` to print an `errors` entry per id the server did not find, exiting 3.
- **BREAKING**: Changed `jmap email parse` to print `bodies` as `[{blobId, body}]`, beside `errors` for the blobs not found or not parsable, exiting 3.
- **BREAKING**: Changed `sieve check` and `sieve put` to print the server's `warnings` as a list of lines, `[]` when none.
- **BREAKING**: Changed `account check` to print `{accounts: [{account, backends}]}`, each backend row `{backend, ok, code, error, tls}`, and exit 3 when one fails, where it exited 0.

  `tls` holds what a backend reached over TLS was configured with, `TLS` in text, and is `null` for a local backend or a cleartext connection.

- **BREAKING**: Changed `account list` to name the backends the build serves under `backends` and the ones it leaves out under `notBuilt`.
- **BREAKING**: Changed the composers' JSON template to carry addresses as envelopes do, `from` an object `{name, email}` or `null` and `to`, `cc` and `bcc` lists of them, where they were strings.
- **BREAKING**: Changed `maildir create|rename|delete` to print the folder's path as `id`, the id the unified `mailbox` commands take, and `maildir list` rows to gain `id`.
- **BREAKING**: Changed `maildir message save` and `mbox message save` to `maildir message add` and `mbox message add`, `save` staying a hidden alias.

  Their JSON schemas are now `himalaya-maildir-message-add` and `himalaya-mbox-message-add`.

- Changed `maildir message copy|move` and `mbox message copy|move` to print the ids the messages got in the target rather than the source ids.
- Changed `imap copy` and `imap move` to print each of `sourceUids` beside the UID it got in `destinationUids`, in `COPYUID` order.
- Changed `imap select` to print the first unseen message's number as `firstUnseen`, `unseen` being a count, and its text to show the recent count, the flags and the highest mod-sequence.
- **BREAKING**: Changed every list to print as plain text, one item per line in columns two spaces apart, with no header row, border, colour or truncation, the way `mscan` prints a mailbox.

  `Label: value` fields (`Account:`, `Mailbox:`, `Page:`, `Next page:`) print above the items and the item errors below, a blank line apart. A message prints its id, the `*` unread, `R` answered, `!` flagged and `@` attachment columns, its date, its senders and its subject.

- **BREAKING**: Changed the records to print as aligned `Label: value` lines: `account check`, `imap select|status|id|fetch`, `jmap vacation-response get`, `sieve capability`, the Gmail and Microsoft Graph `get`s and the header block of `message read`.

  `imap search` and `imap sort` no longer end with a count.

- Changed `--help` to print without colour, each paragraph on one line.
- Changed `attachment download` to print `Saved <size> bytes to <path>` per file written.
- Changed the write confirmations on a pimdir account to say the write is queued (*Message queued for sending*), the sync engine applying it later.
- **BREAKING**: Changed `envelope search` to read the pimdir SEARCH query language (§8), as `pimdir search`, Calendula and Cardamum do: `from:alice date:2026-11-02.. is:unread "weekly sync"`.

  `order by` is now `--sort newest-first|oldest-first|title|sender|relevance`, Maildir and mbox match whole words, and a `~`, `/` or `{` form is reserved (`search-invalid`). A clause the backend cannot search fails `search-unsupported`; MIGRATION.md maps every v2 clause.

- **BREAKING**: Changed `flag add|set|remove` and `message copy|move|delete` given ids the mailbox does not hold to print each as a `message-not-found` entry and exit 3, the others acted on ([#777]).
- **BREAKING**: Changed a send whose follow-up failed to print its receipt and exit 3: a copy refused after the send and a reply's `answered` flag are item errors of the receipt.
- **BREAKING**: Changed a JMAP, Gmail or Microsoft Graph `message read`, and the source read of `message reply|forward` and `attachment list|download`, to fail `message-not-found` when the mailbox given does not hold the message.
- **BREAKING**: Changed `envelope list` and `envelope search` on Maildir and mbox to list a message holding no header field as a `message-unreadable` entry, exiting 3.
- **BREAKING**: Changed `envelope list` and `envelope search` to refuse `--page 0`, which listed every envelope: `--page-size 0` lists the whole mailbox.
- **BREAKING**: Changed `--no-save` of `message compose`, `reply` and `forward` to require `--send`.
- **BREAKING**: Changed `-f/--flag` of `message add`, `imap append`, `imap store`, `maildir message add` and `mbox message add` to take one value per occurrence: repeat `-f` for several flags.
- **BREAKING**: Changed `message add|send`, the protocol saves, `sieve put|check`, `jmap query` and `envelope search` to read an option after their positional as the option, where it became message, script or query text.

  A word opening on a hyphen goes after `--`.

- **BREAKING**: Changed `maildir flag set` and `mbox flag set` to require at least one `-f`, where `mbox flag set` without one cleared every flag.
- **BREAKING**: Changed `maildir message add` to refuse `-f` with `--subdir new`, its default: pass `--subdir cur`.
- **BREAKING**: Changed `--no-save` on a Microsoft Graph send to fail before connecting with `capability-missing`, Graph filing every message it sends in Sent Items.
- Changed a Microsoft Graph send to ignore `--save` and `message.send.save-copy`, and `message add` or a composer's save without `--send` there to fail before connecting with `capability-missing`.
- **BREAKING**: Changed `mailbox list` on Microsoft Graph to leave `total` and `unread` `null` without `--counts`.
- **BREAKING**: Changed `message read --seen` on pimdir to fail when the `\Seen` cannot be staged, where it read the message unseen and exited 0.
- Changed `mailbox list --counts` to list a mailbox whose counts could not be fetched without them, beside a `mailbox-unreadable` entry.
- Changed IMAP flag writes to refuse a keyword the mailbox's `PERMANENTFLAGS` does not allow, coded `capability-missing`.
- Changed `message delete` on an account with neither `mailbox.alias.trash` nor a mailbox the backend marks as the trash to fail with `capability-missing` before moving anything.
- Changed a `--backend` naming a block the account does not configure to fail with `capability-missing`.
- Changed a JMAP send to fail with `capability-missing` before uploading anything when the server lacks `urn:ietf:params:jmap:submission`.
- Changed every build carrying a mail transport to parse the same unified commands, refusing those it cannot serve with `capability-missing`.

  `json-schema` no longer varies with the backends compiled in.

- Changed the shared send over SMTP to connect for each send and close after it.
- Changed `smtp send` to check `--mail-from` and `--rcpt-to` as RFC 5321 mailboxes when the command line is read.
- Changed a PLAIN or SCRAM password to be prepared with SASLprep (RFC 4013) before it is sent or derived from.
- Changed the Gmail listings and writes to batch their requests, 50 messages a request, where they sent one request per message.

  A throttled part is sent again up to three times; a message Gmail refuses is an `errors` entry, the others listed or written.

- Changed the Microsoft Graph writes to patch, copy, move and delete through `$batch`, where they sent one request per message and stopped at the first failure.

  Each message Graph refuses is an `errors` entry coded by its status, the others written.

- Changed the JMAP listings to report an email or a mailbox that does not deserialize as an `errors` entry, where the whole command failed.
- Changed `message send --save`, `message add --send` and a composer's `--send --save` on Gmail to label the message Gmail filed under `SENT`, rather than insert a second message.
- Changed an IMAP account on iCloud to sort client-side when `imap.sort.fallback` is unset.
- Changed the subject and display names of an IMAP envelope to be decoded as `message read` decodes a header, raw 8-bit bytes that are not UTF-8 reading as Windows-1252.
- Changed `message read` to unwrap a `format=flowed` text part (RFC 3676), and a reply to quote it unwrapped.
- Changed `message read` to read malformed but common mail: raw 8-bit headers, a character split between encoded words, UTF-8 labelled `us-ascii`, an invalid `Content-Type`.
- Changed UTF-7 (RFC 2152) text to be shown as written.
- Changed the size `attachment list` reports and the bytes `attachment download` writes to be the part's decoded bytes in its own charset, an attached message written alone.
- Changed the root Maildir to list as `INBOX` with the inbox role, or `.` when a top-level `inbox` folder takes the role, where it listed under its folder's basename.
- Changed a Maildir mailbox to be taken by its name, its absolute path, with `.` segments or a trailing slash, every spelling resolving to one id.
- Changed a `mailbox.alias.<role>` naming no mailbox id to mark the role of the listed mailbox carrying that name, on every backend.
- Changed the mbox backend to refuse a keyword `X-Keywords` would not give back: holding a whitespace, a comma or a control character, or longer than 255 bytes.
- Changed an mbox mailbox name holding a NUL or a `.` or `..` level, or a level starting with `.` or ending in `.lock`, `.lck` or `.msf`, to be refused wherever a mailbox is taken.
- **BREAKING**: Changed the pimdir backend to pimdir storage-v1, sync-v1 and search-v1 (io-pimdir 1.0.0): a store an earlier release wrote is refused `store-version` and its index `index-version`, never migrated.

  Sync it with the neverest that wrote it until `pimdir queue list` lists nothing, move its directory aside, then let a v1 neverest sync a new store (MIGRATION.md).

- Changed a pimdir store a later release wrote to be read while its `read_version` is at most 1, judged by its `store_meta` alone.
- Changed a pimdir `message copy` or `message move` into a mailbox its source does not sync to be applied under a source syncing it, and to park when none does (pimdir SYNC §7).
- Changed a pimdir store whose directory or pimdir.db another user can read to be refused with `store-exposed`: `chmod 700` the directory and `chmod 600` pimdir.db.
- **BREAKING**: Changed the pimdir queue rows to print `row`, the integer row id, with `handle` beside it, the row under the store's identity, where they printed `queueId`.

  `pimdir queue show` and `queue cancel` take the handle, checked against the store, or the bare row id.

- **BREAKING**: Changed `pimdir queue list` to list the whole account, `-m/--mailbox` narrowing it to one, where it listed the inbox when `-m` was omitted.
- **BREAKING**: Changed `pimdir queue list` to print `{actions, parked, errors, refused}`, each row `{row, handle, kind, collection, linkId, itemId, producer, queuedAt, attempts, unconfirmed, error, subject}`.

  Every pending row is listed whatever its kind, the parked ones with why, a row that does not decode under `errors` (exit 3), a change a server refused for good under `refused`.

- Changed `pimdir queue cancel` to take a parked row and print its `kind` and `unconfirmed`, refusing a row it cannot cancel before asking.
- Changed the pimdir writes to be refused before they are queued when a source of the store does not support them, naming the capability, the source and why.

  A send is refused while several sources can send and none is chosen, and coded `no-performer` when no source syncs the mailbox.

- Changed `flag add`, `flag remove` and the `\Seen` a read stages on a pimdir account to state the flags they were read from, so a flag a sync changes meanwhile survives.
- Changed a `message delete` or `message move` queued on a pimdir message a sync left in conflict to be parked by the sync engine rather than applied.
- Changed a pimdir send asking for a copy to carry it in the queued send, so the copy is filed only once the message is sent.
- Changed `message add` and a composer's save on a pimdir account to refuse a message whose `Message-ID` the mailbox holds or has queued, coded `item-duplicate`.
- Changed `message copy` and `message move` on a pimdir account to fail for a message whose body is not local when the account does not sync the target mailbox yet.
- Changed `envelope list` and `envelope search` on a pimdir account to read only what the page needs, where they read every message of the mailbox.

### Deprecated

- Deprecated `-i` for `--inline` on `attachment list` and `attachment download`, `-i` meaning `--interactive` across the family: the help shows `--inline` alone, `-i` still taken as a hidden alias.

### Removed

- **BREAKING**: Removed the configuration wizard: bare `himalaya` running it, the `configure` command (alias `wizard`), the `wizard` cargo feature and the `himalaya-configure` JSON Schema.

  Write the configuration from config.sample.toml, as the first-run welcome shows.

- **BREAKING**: Removed the m2dir backend: the `himalaya m2dir` commands, `-b m2dir`, the `m2dir` cargo feature and its JSON Schemas.

  An `m2dir` block is refused as an unknown key: move to a Maildir or a pimdir store.

- **BREAKING**: Removed the `*.list.table.*-color` options, `table.preset` and `table.arrangement`.

  The `table`, `attachment` and top-level `account` blocks are refused as unknown keys.

- **BREAKING**: Removed the `envelope.list.table.unseen-char`, `replied-char`, `flagged-char` and `attachment-char` options, the `envelope.list.table` block being refused as an unknown key.
- **BREAKING**: Removed `envelope.list.datetime-relative` ([#510]), now refused as an unknown key.
- **BREAKING**: Removed the `HIMALAYA_CONFIG` environment variable: pass `-c "$HIMALAYA_CONFIG"`, one `-c` per path.
- **BREAKING**: Removed `-w/--max-width` from `envelope list`, `envelope search` and `mailbox list`, nothing being truncated anymore.
- **BREAKING**: Removed `tmp` from the `--subdir` values of `maildir message add|copy|move`.
- **BREAKING**: Removed `deleted`, `deletedStorage` and `highestModSeq` from `imap status`, which never requested them and printed them `null`.

### Fixed

- Fixed a `mid:` search over IMAP finding nothing on iCloud: the id is sent in angle brackets, as is the UID lookup of an `APPEND` answered without `APPENDUID`.
- Fixed `envelope list` over JMAP leaving the order to the server: it asks for the newest first, as `envelope search` does.
- Fixed a `message copy` or `message move` on a pimdir account joining a second message into a mailbox another source holds it in already.

  It needs a neverest on the same io-pimdir.

- Fixed the text error report writing the control characters of its causes raw to the terminal.
- Fixed `--log-file` creating the file under the umask and following a symbolic link: it is created at 0600 and a link is refused.
- Fixed `himalaya manual` printing `vv<version>`, leaving out the pages of the nested commands and pointing SEE ALSO at pages it did not write.
- Fixed a configuration parse error printing its TOML source line, and a refused secret echoing its value.
- Fixed a failing secret command saying nothing: it names its program and exit status, never its output ([#745]).
- Fixed a `tls.cert` that cannot be opened failing with a bare `No such file or directory`: the command fails before connecting, naming the file.
- Fixed install.sh exiting 127 after a successful install and saving an HTTP error page as the archive.
- Fixed a JMAP date search missing mail whose `Date:` carries an offset east of UTC.
- Fixed `message read`, `attachment list|download`, `message reply` and `message forward` on IMAP sending `SELECT sent` for a role: the mailbox resolves as `envelope list` resolves it.
- Fixed the Gmail envelopes splitting an address header on a comma inside a quoted name (`"Doe, John" <john@x.org>`).
- Fixed `gmail labels update` clearing the label's colour and visibility.
- **BREAKING**: Fixed the IMAP `ENVELOPE` group markers printing as addresses in `envelope list`, `envelope search` and `imap fetch`: `To: undisclosed-recipients:;` lists no recipient.
- Fixed `attachment download` writing parts before refusing an id the message does not hold, and failing on a name past 255 bytes, now cut at a character boundary.
- Fixed `attachment download` keeping control and bidirectional formatting characters in a file name, which can disguise an extension, and naming a part `. .` as `.`.
- Fixed `attachment download` creating its files readable by others: they are created 0600.
- Fixed `attachment download` following a dangling symlink, or overwriting an existing file once the name suffixes ran out.
- Fixed `attachment download` without an id writing the inline parts `attachment list` leaves out: they are written with the new `--inline` only.
- Fixed saving the mbox cache taking time quadratic in the mailbox.
- Fixed the mbox envelope cache being readable by every user of the host: its directory is kept at 0700 and its files at 0600.
- Fixed a Maildir write reading the folder once per id: one read looks them all up.
- Fixed the Maildir and mbox listings holding every message of the mailbox in memory.
- Fixed each JMAP blob download on another host running the credential command again and opening a new connection.
- Fixed every Microsoft Graph command naming a mailbox listing the whole folder tree first.
- Fixed a Gmail search flattening an `and` under an `or` into alternatives: an alternative holding more than one term is wrapped in parentheses.
- Fixed `envelope list|search --page-size 0` on Gmail reading the first 100 messages and on Microsoft Graph 10: every page is read.
- Fixed `pimdir queue cancel` cancelling a row of another account or app sharing the store.
- Fixed `pimdir queue cancel` reshaping the store's schema to Himalaya's io-pimdir revision, dropping what another app's revision keeps.
- Fixed `imap raw` panicking on a literal holding a byte that is no UTF-8, and splitting such a literal into responses.
- Fixed a mistyped path, such as one holding a colon, being stored as a message by `message add` and the other commands taking a message: a value naming no file must open on a header field.
- Fixed `sieve activate ""` deactivating every script: an empty name is refused, `sieve deactivate` deactivating.
- Fixed a move onto its own mailbox writing anyway on mbox, Maildir, IMAP and pimdir: it changes nothing and counts the messages.
- Fixed a `Date:` naming the wrong weekday reading as no date on Gmail and misplacing the message in IMAP's client-side date sort.
- Fixed `mailbox list --counts` on Microsoft Graph counting a folder's contacts and events with its messages.
- Fixed an IMAP `message delete` in the trash that found no id reporting `flagged` and advising an expunge ([#814]).
- Fixed an IMAP listing dropping a message whose `FETCH` line does not decode: it is listed with the items that decoded, beside a `message-unreadable` entry ([#801]).
- Fixed a Maildir listing skipping an unreadable message file or folder with a warning only: each is a `message-unreadable` or `mailbox-unreadable` entry.
- Fixed the Microsoft Graph lookups counting every 400 as an id or a folder not found.
- Fixed a send over SMTP running again the credential command the IMAP block already ran ([#813]).
- Fixed a send taking from its headers an envelope address holding a control character or a line break: it fails naming the address before anything is written.
- Fixed `scram-sha-256` on IMAP and SMTP failing as an unsupported mechanism.
- Fixed an IMAP answer holding lines that do not decode being printed as whole: a warning counts the skipped lines, and a server `ALERT` is logged.
- Fixed an IMAP command line holding a literal being sent before the server's go-ahead.
- Fixed an IMAP greeting carrying a literal panicking.
- Fixed IMAP commands failing to decode a `NO` with UTF-8 in its text, such as Dovecot naming a non-ASCII mailbox.
- Fixed mbox writes and listings failing, or editing the wrong bytes, on an index another client made stale: the index is rebuilt and the operation tried once more.
- Fixed a JMAP send the server refused leaving the imported message in the drafts mailbox.
- Fixed a JMAP send saying nothing when the server sends the message but leaves it in the drafts: a warning says so.
- Fixed a JMAP submission refused as `tooLarge`, `forbidden`, `overQuota` or `rateLimit` reading as `unknown`.
- Fixed a Gmail label type or setting value Himalaya does not know failing the whole answer.
- Fixed `message read` printing unquoted a MIME parameter value that is no RFC 2045 token.
- Fixed `mailbox list --counts` on pimdir leaving `unread` `null`.
- Fixed malformed multiparts reading with the wrong structure, such as three attached messages read as one; part ids follow the corrected structure on such mail.
- Fixed a header field whose name holds a blank being read as the field it spells, and a body line being read as `From:` after an unusual `Date` zone.
- Fixed `flag add|set|remove` with `answered` or `draft` on Gmail and Microsoft Graph, and `message add -f answered|draft` on Gmail, reporting success while changing nothing: they fail with `capability-missing`.
- Fixed `flag set` on Gmail failing whole by naming the `DRAFT` label.
- Fixed Gmail date clauses being off by a day: each bound is sent as the Unix seconds of the UTC day.
- Fixed a JMAP date clause under `not` or `or` being checked as if it stood alone: it reads the server's received date.
- Fixed `hasAttachment` reading differently per backend: IMAP, Maildir, mbox and pimdir follow one rule ([#802]).

  Any disposition but `inline` counts; with none, a part counts unless text, multipart or named by a `Content-ID`; a signature or an encrypted part never does.

- Fixed `account list` never naming `pimdir` among an account's backends.
- Fixed `account check` refusing an account declaring only a pimdir block: the store is opened, its mailboxes listed and a producer probed.
- Fixed a pimdir add or send writing its body before the store accepted the write, so a collector could delete it or a refused write leave it behind.
- Fixed `body` search clauses never matching on a pimdir account.
- Fixed a pimdir flag change queueing a row for a message that already shows the result.
- Fixed `flag add` and `flag remove` on a pimdir account replacing every flag of a message whose flags the sync engine never read: they fail saying so.
- Fixed `flag remove` on a pimdir account missing a flag the store holds in another spelling (`\seen`, Thunderbird's `Junk`).
- Fixed a pimdir write on several messages staging the first ones before failing on a later one: the rows are one batch, all staged or none.
- Fixed Maildir mailboxes being named by their last path segment: they are named by their path relative to `maildir.root` ([himalaya-tui#22](https://github.com/pimalaya/himalaya-tui/issues/22)).
- Fixed control and bidi characters from messages and servers reaching the terminal through the plain output.
- Fixed pimdir queue details in the unified outputs: `envelope list` reports no `queued` count and a send prints no `queueId`, `pimdir queue list` showing what is queued.
- Fixed `message compose`, `reply` and `forward` writing a `Message-ID` on the machine's hostname: it sits on the domain of the `From` address.
- Fixed `message copy|move` on IMAP without MOVE reporting a copied message as not found when the `COPYUID` cannot be read, such as Mailo's UIDVALIDITY 0 ([#781]).
- Fixed a raw message piped on stdin being cut at its first line that is not valid UTF-8 and still reported as sent or saved: stdin and message files are read whole as bytes ([#786]).
- Fixed the plain output of the writes, `sieve get|list|capability` and `jmap query` not ending with a newline, and `jmap email parse` running its bodies together ([#777]).
- Fixed the `envelope search` query errors carrying ANSI colour escapes, even inside the `--json` error string.
- Fixed `message delete` in a JMAP trash destroying an email other mailboxes held too.
- Fixed the writes on JMAP, Gmail and Microsoft Graph acting on an id of another mailbox: an id counts only when its message is in the mailbox given.

  On Graph, `message delete -m trash` given the id of a message in another folder deleted it for good.

- Fixed `message delete` in the trash reporting every id given as deleted: it counts the ids the trash held.
- Fixed `flag add|set|remove` and `message copy|move|delete` running on no id, and acting twice on a repeated one.
- Fixed Maildir and mbox writes given an unknown id among known ones failing part-way: every id is looked up before the first write.
- Fixed a Maildir store listing failing on a plain file beside its folders (dovecot-uidlist), and listing phantom mailboxes through a symlink back to a folder.
- Fixed the Maildir info flags being written out of ASCII order (`:2,SF` for `:2,FS`), and an experimental `:1,` info section being read as flags.
- Fixed a missing `maildir.root` or `mbox.root` listing as an empty store: it fails naming the root.
- Fixed the Maildir and mbox mailbox writes taking a blank name or one holding a control character, and creating over or renaming onto an existing Maildir.
- Fixed `maildir delete` and a Thunderbird-layout `mbox delete` removing the mailboxes held under the one deleted, and the Maildir root being deletable: both are refused.
- Fixed a Thunderbird-layout mbox rename orphaning the `.sbd` directory of its children: it moves with the mbox.
- Fixed an mbox flag write leaving Thunderbird's `X-Mozilla-Status` and `X-Mozilla-Keys` out of step.
- Fixed mbox reads carrying the `Status`, `X-Status`, `X-Keywords`, `X-Mozilla-*` and `Content-Length` fields the store writes, so `message read --raw` gives back the message added.
- Fixed the mbox commands deleting or renaming the spool the mail system delivers to: it is refused, as IMAP refuses it for `INBOX`.
- Fixed `INBOX` on mbox without `mbox.inbox` addressing a missing file of that name below the root: it fails naming `mbox.inbox`.
- Fixed mbox listing and appending to files that are no mbox: a non-empty file not opening on a `From_` line is left out and refused.
- Fixed an mboxcl or mboxcl2 `Content-Length` running past a message boundary merging the messages after it under one id.
- Fixed a busy mbox lock failing uncoded: a lock held past `mbox.lock.timeout` fails `store-busy`, and a denied dotlock names `mbox.lock.dotlock = false`.
- Fixed `message delete` on Maildir moving the trash's messages to the trash again when the trash was named by path or alias.
- Fixed a copy or move between Maildirs carrying dovecot keyword letters verbatim: with `maildir.keywords.dovecot` they are renamed through the target's file.
- Fixed a Maildir that cannot be searched reading as `missing cur/ subdirectory`: the permission error names the path.
- Fixed the local `body` search clause matching case-insensitively in ASCII only.
- Fixed the client-side sort of Maildir, mbox, pimdir, Gmail, Microsoft Graph and IMAP's fallback to follow RFC 5256: base subject, first address's mailbox, case ignored.
- Fixed an IMAP keyword search naming a registered keyword in another case being sent as typed: `junk` sends `KEYWORD $Junk`.
- Fixed IMAP searches negating a group or an address clause, which iCloud answered wrongly: negations are pushed onto single keys.
- Fixed a UID Fastmail's `SORT` lists twice being listed twice.
- Fixed `imap list` sending a non-ASCII pattern unencoded and an `LSUB` pattern unquoted, which iCloud refuses.
- Fixed `imap fetch --structure` printing the server's uppercase MIME types.
- Fixed `imap copy` and `imap move` saying the messages were copied or moved when no `COPYUID` said so.
- Fixed `imap thread` on a server without the algorithm's `THREAD=` capability failing with a parse error: it is refused with `capability-missing`.
- Fixed a JMAP write failing whole on one refused id: each is an `errors` entry coded by its SetError type, the others written.
- Fixed `message add`, a save and a send on a JMAP server forbidding duplicates (Fastmail) failing on bytes the account holds: the existing email is filed or submitted.
- Fixed the JMAP set and method errors dropping the type, description and arguments the server sent.
- Fixed setting the JMAP vacation response on Fastmail failing after the update was applied.
- Fixed a JMAP mailbox name matching none being sent to the server, and one two mailboxes carry resolving to the first: both are refused.
- Fixed `jmap email read` printing server values unsanitized, running two emails together and exiting 0 when no id named one.
- Fixed the JMAP `query` commands returning the objects in the server's order rather than the query's.
- Fixed `jmap email update` dropping the patches a replacement overrode and accepting no change: both are refused.
- Fixed a Gmail search pattern widening the search with the operators it held: each pattern is a literal.
- Fixed a Gmail reply or forward added or sent outside its thread: it carries the parent's `threadId`.
- Fixed a Gmail mailbox naming no label failing with Gmail's error: it is refused naming the value.
- Fixed the Gmail snippets printing their `<b>` marks and HTML entities.
- Fixed Gmail ids, label ids and addresses reaching the request paths unencoded.
- Fixed the Microsoft Graph roles dropping at random, `-m sent` then reaching Graph as a folder id.
- Fixed `mailbox list` on Microsoft Graph leaving out the nested folders and every folder past the first page: a nested one is named by its path (`Projets/Alpha`).
- Fixed a Microsoft Graph mailbox naming no folder being sent into the request path: it is refused, and every id is percent-encoded.
- Fixed the Microsoft Graph envelopes being dated by arrival: the date is the `Date:` header, else the arrival.
- Fixed a Microsoft Graph search pattern holding KQL syntax widening the search: each word is a quoted KQL phrase.
- Fixed `flag set` on Microsoft Graph leaving the categories it did not name.
- Fixed `msgraph messages list --search` failing on a value holding `:` or `-`.
- Fixed `msgraph attachments list` listing the first page alone: it follows every next link.
- Fixed an attached `message/rfc822` being written base64, which RFC 2046 forbids, and a forward dropping the `Content-Type` parameters of the source's attachments.
- Fixed a composed single-part message not ending with a line break, a quoted empty line carrying a trailing blank, and the reply or forward JSON template carrying CRLF line breaks.

### Security

- Bumped rustls to 0.23.46, fixing RUSTSEC-2026-0346 to RUSTSEC-2026-0349: TLS secrets exposed by `Debug`, a downgrade to classical key exchange, and an unbounded certificate decompression.
- Bumped ctutils to 0.4.6, rustls-webpki to 0.103.16 and webpki-root-certs to 1.0.11, fixing RUSTSEC-2026-0382, RUSTSEC-2026-0383 and RUSTSEC-2026-0385.
- Fixed IMAP `LOGIN` and `AUTHENTICATE` logging credentials at trace level.
- Fixed a server making Himalaya buffer without end: an HTTP response head is capped at 64 KiB, a body at 256 MiB, and a `STARTTLS` response line is capped too.
- Fixed a SCRAM server announcing fewer than 4096 or more than five million iterations being accepted.

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
[#528]: https://github.com/pimalaya/himalaya/issues/528
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
[#801]: https://github.com/pimalaya/himalaya/issues/801
[#814]: https://github.com/pimalaya/himalaya/issues/814
[#802]: https://github.com/pimalaya/himalaya/issues/802
[#805]: https://github.com/pimalaya/himalaya/issues/805
[#806]: https://github.com/pimalaya/himalaya/issues/806
[#813]: https://github.com/pimalaya/himalaya/issues/813

[core#1]: https://github.com/pimalaya/core/issues/1
[core#10]: https://github.com/pimalaya/core/issues/10

[unreleased]: https://github.com/pimalaya/himalaya/compare/v3.0.0...HEAD
[3.0.0]: https://github.com/pimalaya/himalaya/compare/v2.2.1...v3.0.0
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

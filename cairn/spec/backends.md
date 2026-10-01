---
cairn: spec
capability: backends
status: current
---

# Backends

Each backend is a `<Proto>Client` wrapper that derefs onto the io-* `*Std` client, paired with a `src/<proto>/backend.rs` adapter implementing the shared operations over the client's high-level methods and converting io-* results into the CLI's own `email` shared types (`Address`, `Envelope`, `Flag`, `Mailbox`, and the search query). The CLI owns these types; no aggregator library sits between it and the io-* crates.

### Requirement: Shared operation set
The shared adapters SHALL cover, per backend: `list_mailboxes`, `list_envelopes`, `search_envelopes`, `store_flags`, `get_message`, `add_message`, `copy_messages`, `move_messages`, and `send_message`. A backend that cannot model an operation opts out of it rather than emulating it.

### Requirement: The envelope carries its threading pointers
The shared `Envelope` SHALL carry `message_id` and `in_reply_to`, the RFC 5322 §3.6.4 identity of a message and of the message(s) it replies to, so a client can pair a reply with its parent from a listing rather than by reading bodies.

`in_reply_to` SHALL be a list, the grammar being `1*msg-id`, and every id in it SHALL be normalised exactly as `message_id` is (angle brackets and surrounding whitespace stripped), so the two compare byte-for-byte whatever backend surfaced them.

Each backend SHALL source the field from the response its listing already makes, and SHALL leave it empty rather than issue a request of its own: IMAP from the `ENVELOPE` (RFC 3501 §7.4.2, 9th element), JMAP from the `inReplyTo` property of `Email/get`, Gmail from the metadata headers, Maildir, m2dir and mbox from the parsed message, and pimdir from the stored summary. Graph leaves it empty, `In-Reply-To` living in `internetMessageHeaders`, which a listing selection does not return.

The field SHALL NOT take a column in the `envelope list` table, where a column of raw msg-ids would be noise; it rides the JSON output.

### Requirement: Network backends
IMAP, JMAP, Gmail and Microsoft Graph SHALL each adapt their io-* high-level client. IMAP reuses io-imap's `select`/`fetch`/`store`/`copy`/`move`/`append`/`list`/`status`. JMAP reuses io-jmap's `mailbox_get`/`email_query`/`email_get`/`email_set`/`email_import`/`email_submission_set`/`blob_upload`/`blob_download`, addressing mailboxes by their JMAP id. Gmail treats labels as mailboxes over io-gmail's `labels`/`messages` surface; Graph treats mail folders as mailboxes over io-msgraph's `mail_folders`/`messages` surface.

### Requirement: IMAP move without the MOVE extension
The IMAP adapter's `move_messages` SHALL use `UID MOVE` (RFC 6851) when the server advertises `MOVE`. Otherwise it SHALL `UID COPY` the set to the target, then flag `\Deleted` in the source. When the copy returns `COPYUID`, those source UIDs SHALL be the ones flagged and, when the server advertises `UIDPLUS` (RFC 4315), `UID EXPUNGE`d. Without `UIDPLUS` it SHALL skip the expunge, leave the messages flagged in the source, and log that at debug. Nothing is flagged or expunged when the copy affected nothing. A plain `EXPUNGE` SHALL NOT be issued, so unrelated `\Deleted` messages stay.

### Requirement: Network transport resilience
The network backends SHALL run over a transport that retries a stream reporting it is not ready (`EAGAIN` on Unix, `EINTR`, and the Windows spelling of an expired deadline) rather than ending the exchange on it. Each read and each write carries its own budget of one minute, so a slow but progressing transfer never runs out of it, and exhausting the budget SHALL fail with a message naming it rather than a raw errno.

Opening a connection SHALL arm a socket read deadline matching that budget, so a server going silent on an otherwise healthy connection ends the command instead of blocking forever.

### Requirement: Local storage backends
Maildir, m2dir and pimdir SHALL adapt io-maildir, io-m2dir and io-pimdir. Maildir stores added messages under `cur/` and SHALL read an entry's flags through io-maildir rather than parsing the filename itself, so the meaning of a Maildir name is decided in the library that owns the format. m2dir is content-addressed with no native copy or move, so those are a get plus a store (plus a delete for move), and its flags live in a `.meta/<id>.flags` sidecar. m2dir mailbox `rename` and message `copy`/`move` remain unavailable until io-m2dir supports them. pimdir is an offline cache the sync engine populates, io-pimdir implementing both the store and the engine: reads project the store's items with their typed mail summaries (pimdir STORAGE Annex A) without body reads, and writes are actions queued for the store's owner to apply and a later sync to propagate, never direct SQL.

### Requirement: mbox backend
The mbox backend SHALL adapt io-mbox over the full shared operation set. A mailbox is one mbox file: `mbox.root` is walked recursively, the mailbox `a/b` being the file `a/b` below it (`a.sbd/b` with `mbox.thunderbird`), and `mbox.inbox`, when set, is the spool shown as `INBOX` and marked with the inbox role. An absolute path passed as a mailbox SHALL open that file, so any mbox reads without configuring it. The raw `mbox` command SHALL expose create, rename, delete and list of mbox files, message save, copy and move, and the six flag letters of the `Status` and `X-Status` headers.

### Requirement: An mbox message is its content id
The mbox backend SHALL address a message by its io-mbox content id, a hash of the message with the flag and metadata fields left out, so a flag write by Himalaya or any other client keeps it. An id the index no longer holds SHALL fail naming the mailbox. An append SHALL report the id the synced index gives the new message, numbered like any duplicate.

### Requirement: mbox flags interoperate
Shared flags SHALL map onto `Status: R` (seen), `X-Status: A`, `F`, `T` and `D` (answered, flagged, draft, deleted), any other flag going to `X-Keywords`. A message marked seen SHALL be marked old (`O`) too, as mutt writes it, since GNU mail lists a `Status: R` alone as new. A shared `flag set` SHALL keep the `O` a message carries, no shared flag naming it.

### Requirement: mbox writes lock and rewrite in place
Every mbox write SHALL go through io-mbox under the dotlock and the fcntl lock, each one skippable through `mbox.lock.dotlock` and `mbox.lock.fcntl`, waiting `mbox.lock.timeout` seconds for a busy one. Flag changes and removals SHALL rewrite the file in place, the whole set of messages a command names in one rewrite. A move SHALL copy then remove, so an interruption leaves the messages in both files.

### Requirement: mbox reads go through a cache
The mbox backend SHALL keep, per file, the io-mbox scan index and the envelope of every message already parsed, under `<XDG cache>/himalaya/mbox/`. Every operation SHALL sync the index first, so an unchanged file is not scanned and a delivery is scanned alone, and a listing SHALL parse only the messages the cache lacks. The cache SHALL be disposable: a missing, stale or unreadable one is rebuilt, failing to write one is not an error, and a read finding its entry stale SHALL rebuild the index and retry once.

### Requirement: Maildir surfaces custom keywords on demand
The Maildir backend SHALL surface custom (non-IANA) keywords on read when told which convention the mailbox uses, so a keyword written by dovecot, mbsync, OfflineIMAP, mutt or notmuch matches a `flag <name>` search as it does on the network backends. `maildir.keywords.dovecot` SHALL resolve the lowercase info-section slot letters through the resolved mailbox's own `dovecot-keywords` file, and `maildir.keywords.header` SHALL read keywords from `X-Keywords` (comma-separated) or `X-Label` (space-separated). Both default to off, and with both off the flag set SHALL be exactly the six standard info-section letters as before.

Keyword reading is not a round trip: no command can name a custom keyword, so a `FlagOp::Set` store SHALL replace the whole set and drop any keyword the message carried.

A sidecar that is absent, unreadable or disabled SHALL yield no keywords rather than fail the listing, since a mailbox without one is the normal case rather than an error.

### Requirement: A Message-ID is not an address
The pimdir backend SHALL NOT assume an item's link id is the `Message-ID` its body carries, nor that a `Message-ID` identifies at most one message in a mailbox. A store may hold two messages of one mailbox sharing a `Message-ID`, keyed apart by the store (pimdir SPEC §9), and both SHALL list, read and act as ordinary messages, each with its own public id and neither marked.

What stays unique is the key and the public id: `(collection, link_id)` still names one item and `seq` still names one message. What ends is the link id being derivable from the body, so a read that re-derives an identity in order to address a row is addressing an unknown number of them.

A mailbox holding one `Message-ID` twice is ordinary (a double delivery, a retried append, a restore, a copy of a sent message), and the store now keeps both rather than one. Showing one of them, or resolving an identity to whichever row came first, hides a message the server holds.

### Requirement: pimdir shows a short public id
The pimdir backend SHALL show and accept each message's public id (`items.seq`, a small store-assigned integer, the same across every mailbox the message is filed in) as its `Envelope.id`, not the internal `link_id`. It SHALL check the id against the collection before reading a body or staging an action, and SHALL fail clearly on a non-numeric or unknown one. `add_message` SHALL return the link id it staged: a queued create has no `seq` yet, the store assigning one when its owner applies the action.

Addressing by the public id is what keeps two duplicated messages distinguishable: they carry one `Message-ID` between them and have two `seq`s, so an address derived from the body would be ambiguous where a `seq` is not.

### Requirement: A pimdir mailbox is its collection id
The pimdir backend SHALL show and accept a mailbox as the store's collection id, verbatim: the collection `imap/INBOX` is the mailbox `imap/INBOX`. It SHALL NOT derive, strip or accept a shortened spelling, and no configuration SHALL offer one.

The id is opaque to the store, which neither parses nor validates it (pimdir SPEC 9.2) and models hierarchy through `parent` rather than through a separator. Any shortening is therefore a guess at the producer's convention, and one that makes a single mailbox answer to two spellings. This is the JMAP backend's shape, whose ids are opaque server strings, and `[mailbox.alias]` is the shortcut for both.

`Mailbox.name` SHALL carry the collection row's own name rather than one derived from the id.

A mailbox is a collection declared `message/rfc822`, matched on the bare media type. A collection declaring no kind is not a mailbox: the store cannot summarise it, and the draft that left kinds undeclared is refused on open rather than read.

A mailbox matching no collection of the account SHALL be refused naming the ones it holds. It SHALL NOT be passed to the store unresolved, which reads as a mailbox that exists and is empty.

### Requirement: pimdir reads one account
The pimdir backend SHALL show the collections of one account (pimdir SPEC §9.2), `pimdir.account` naming it. Unset, it is derived: a store holding one account, or one ungrouped set, is read as that one, and a store holding several is refused naming them rather than guessing one and showing the wrong mailbox set.

### Requirement: pimdir is a reader and a producer, never the owner
The pimdir backend SHALL treat the store as a possibly-partial cache owned by the sync engine. `get_message` on an item whose body is not local (`level < Full`, no stored object) SHALL report a clear "body not fetched" state (the cue to sync), not a data-loss error; the item still lists.

Reads SHALL go through `PimdirReader`, the role that takes no lock (pimdir SPEC §8) and carries no write at all, so a sync in flight neither blocks Himalaya nor is blocked by it, and the backend cannot drain the queue or sweep the store even by mistake.

An envelope SHALL be built from the item's typed mail summary and the addresses joined to it (pimdir STORAGE Annex A.1, A.6): every `From` and `To` address with its display name, the `In-Reply-To` list, the date, the size and the attachment flag, with no body read. A listing SHALL take the store's own order, newest first, the mail sort key being the date.

The reader SHALL overlay the queue (pimdir SPEC §15.4), so an action this client staged is visible on the next read rather than on the next sync: a staged `set-flags`, `update`, `remove`, `move` or `copy` changes what a listing shows. Each addresses a message that already exists and keeps its public id, so a staged write never changes how a message is addressed.

A write SHALL be staged as a queued `PimdirAction` through a producer handle (`store_flags`→`SetFlags`, `add_message`→`Add`, `copy_messages`→`Copy`, `move_messages`→`Move`, `delete_messages`→`Remove`), addressed by the public `seq`, for the store's owner to apply and push. The backend SHALL NOT write the index, load a collection, or run the owner's object sweep: a sweep run beside a sync destroys the bodies it has streamed but not yet attached, which SPEC §14 explicitly invites it to leave pending. A body an action references SHALL be written to the blob store durably before the action is enqueued, the queue row being what pins it.

`SetFlags` carries the whole replacement set, so applying it twice lands the same state; a set the store reports as unknown contributes no markers rather than staging an unknown one, which would erase what a sync knows. pimdir has no native trash.

An added message SHALL derive its link id through io-pimdir's mail derivation (`summary::mail::derive`), the one implementation of SPEC Annex A.1, which is the bare `Message-ID` with nothing prepended, and SHALL name it on the queued `Add`. The action carries no summary: the owner derives the summary and the sort key from the body when it applies the action, through the same call, so the two never disagree. A staged `Add` whose link id the collection already holds SHALL park (pimdir SPEC §15.3): it neither deduplicates against the stored copy nor mints a second key. Minting is the store's answer to what a source hands over; parking is its answer to a producer authoring a message the collection already has.

### Requirement: A queued creation is reported, not listed
A queued creation has no public id until the store's owner applies it, so the pimdir backend SHALL NOT project one as an envelope, and SHALL NOT put a placeholder in `Envelope.id`. `add_message` returns the link id it staged, which identifies the creation across the window.

An envelope listing SHALL report how many creations the mailbox has queued and name the command that shows them, so a saved message that is not in the list reads as queued rather than as lost. A backend that stages nothing reports none, which every backend whose writes reach the server as they are made does. An envelope *search* SHALL report none whatever the backend: a queued creation is never matched against the query, so a count its filter never saw would be misleading.

### Requirement: The pimdir subcommand reads and retracts the queue
Himalaya SHALL carry a `pimdir` subcommand for what the operator CLI cannot do without knowing mail. `queue list` SHALL render a queued creation as a message (flags, subject, recipient, and when it was queued, from the row's `created_at`) where the kind-agnostic `pimdir` binary can only print ids and hashes. The queued action carries no summary, so the row SHALL be derived from the body the action pins, through the same derivation the owner applies; an action pinning no body renders with its flags alone. `queue cancel` SHALL retract one row through io-pimdir's scoped owner operation, confirming first unless `--yes`.

Taking the owner role briefly is what cancelling costs (pimdir SPEC §15.5); the backend read and write paths SHALL NOT reach it. A store another process owns SHALL be refused immediately, saying a sync is running and that the action may already have been applied.

#### Scenario: A queued creation is rendered from its body
- GIVEN a queued `add` pinning a body whose headers carry a subject and a recipient
- WHEN the queue is listed
- THEN the row shows that subject and recipient, and an empty message id

### Requirement: pimdir sends by queueing a submit intent
The pimdir backend SHALL send a message by staging its raw bytes and enqueueing one `submit` action, which whatever owns the store performs through its own channel. It SHALL NOT open a network connection to send, and SHALL NOT use the account's `smtp` section.

The payload SHALL be `v: 1` JSON carrying `object` (the body hash), `from`, `rcpts` and `subject`, derived from the headers by the same function the SMTP adapter uses: the first `From:` address, every `To:`, `Cc:` and `Bcc:` address, and the decoded subject. A message with no `From:` or no recipient SHALL be refused before anything is staged. The body SHALL be stored as given, `Bcc:` included; removing it is the sending channel's job.

The row SHALL anchor on the `--save` mailbox when one is given, else on the mailbox `mailbox.alias.sent` names; with neither, the send SHALL be refused, naming the alias to set, rather than create a collection.

The command SHALL report the queue row id, and its text SHALL say the message is queued for sending, not sent.

#### Scenario: A send made offline waits in the queue
- GIVEN a pimdir account with `mailbox.alias.sent` set and no network
- WHEN a message is sent
- THEN one `submit` row is queued on the sent mailbox with the message's envelope, and the command prints its row id

#### Scenario: A Bcc recipient is in the envelope
- GIVEN a message with `To: a@x.org` and `Bcc: b@x.org`
- WHEN it is sent on a pimdir account
- THEN the payload's `rcpts` holds both addresses and the stored body still carries `Bcc:`

#### Scenario: No anchor is refused
- GIVEN a pimdir account with no `mailbox.alias.sent`
- WHEN a message is sent without `--save`
- THEN nothing is staged and the error names `mailbox.alias.sent`

### Requirement: The queue view shows queued sends
`himalaya pimdir queue list` SHALL render a queued `submit` as a message, derived from the body it pins as a create is, marked as a send, beside the queued creates of the same mailbox. The count an envelope listing reports SHALL include the mailbox's queued sends.

### Requirement: Append and search gaps
Gmail and Graph SHALL NOT implement `add_message` (neither API has an append) and SHALL NOT implement shared `search_envelopes`. IMAP, JMAP, Maildir, m2dir and mbox implement search (see the search capability).

The shared `add_message` result SHALL carry an optional id, absent only when an IMAP or JMAP server acknowledges the write without reporting one. An IMAP `APPEND` acknowledged by the server SHALL stay a success when neither `APPENDUID` (RFC 4315) nor the fallback `Message-ID` search yields a UID, and a requested send SHALL go on. An `APPEND` rejected by the server SHALL stay an error.

### Requirement: Sending transport
Backends that self-send (JMAP, Gmail, Graph) SHALL route `send_message` through their own API, and pimdir through its queue. Storage backends that cannot send (IMAP, Maildir, m2dir, mbox) SHALL send through the account's SMTP transport, adapted in `src/smtp/backend.rs` over io-smtp, which parses the RFC 5321 envelope from the raw message headers. The transmitted message SHALL NOT carry the `Bcc:` field (RFC 5322 3.6.3), which io-smtp removes after the envelope is derived. The protocol-level `smtp send` SHALL transmit its message verbatim, its envelope being explicit.

### Requirement: A JMAP send files the message as sent
A JMAP send SHALL stage the message in the drafts mailbox with `$draft`, then submit it with an `onSuccessUpdateEmail` patch (RFC 8621 section 7.5) moving it to the sent mailbox, unsetting `$draft` and setting `$seen`. The drafts mailbox SHALL be `jmap.drafts-mailbox-id`, else the `drafts`-role one, the send failing without either; the sent mailbox SHALL be `jmap.sent-mailbox-id`, else the `sent`-role one, the message only losing `$draft` without either.

### Requirement: Mailbox role
A shared mailbox SHALL carry an optional role (inbox, all, archive, drafts, flagged, important, junk, sent, subscribed, trash, or a verbatim unknown one), shown by `mailbox list` in its table and JSON output. JMAP reads it from the mailbox `role`, Gmail from its fixed system-label ids, Microsoft Graph from its well-known folder names resolved to folder ids in one `$batch` request, and IMAP marks only `INBOX`, as mbox marks its spool when one is configured. Maildir, m2dir and pimdir have no native role. A `mailbox.alias.<role>` entry overrides the native role of the mailbox it names, and `message delete` takes its trash from that alias before the backend's trash role.

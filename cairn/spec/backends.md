---
cairn: spec
capability: backends
status: current
---

# Backends

Each backend is a `<Proto>Client` wrapper that derefs onto the io-* `*Std` client, paired with a `src/<proto>/backend.rs` adapter implementing the unified operations over the client's high-level methods and converting io-* results into the CLI's own `email` shared types (`Address`, `Envelope`, `Flag`, `Mailbox`, and the search query). The CLI owns these types; no aggregator library sits between it and the io-* crates.

### Requirement: Unified operation set
The shared adapters SHALL cover, per backend: `list_mailboxes`, `create_mailbox`, `rename_mailbox`, `delete_mailbox`, `list_envelopes`, `search_envelopes`, `store_flags`, `get_message`, `add_message`, `copy_messages`, `move_messages`, and `send_message`, and JMAP and Gmail `get_vacation` and `set_vacation`. A backend that cannot model an operation opts out of it rather than emulating it, declaring the capability `none` in its table (cairn/spec/commands.md), so the resolver refuses it before connecting.

### Requirement: A copy on JMAP and Gmail is one message in two mailboxes
`message copy` on JMAP (a mailbox added to `mailboxIds`) and Gmail (a label added) SHALL keep one message in both mailboxes, sharing flags and deletion, where IMAP, Graph and the local backends make an independent copy; both declare `message.copy` `partial` with that detail. `jmap email copy` (`Email/copy`, across accounts) stays protocol-specific.

### Requirement: Gmail and Graph declare no answered or draft flag
Gmail and Graph SHALL declare `flags.answered` and `flags.draft` `none`: Gmail has no label for `\Answered` and refuses `DRAFT` from a client, Graph has no answered field and its `isDraft` is read-only. A shared flag command naming either SHALL be refused before any request, never reported done.

### Requirement: An IMAP keyword is kept or refused
An IMAP flag write SHALL refuse, under `capability-missing` and before the `STORE`, a keyword the selected mailbox's `PERMANENTFLAGS` lists neither by name, compared ignoring ASCII case, nor by `\*` (RFC 3501 section 7.1). A server sending no `PERMANENTFLAGS` SHALL be taken to keep every flag. The system flags are left to the server.

### Requirement: Unified outputs carry no backend details
The output of a unified command SHALL NOT carry a field that only one backend fills. A backend detail worth showing SHALL be logged by that backend's adapter or shown by its own namespace.

### Requirement: Backends reading the MIME structure mark attachments by the pimdir rule
IMAP, Maildir, m2dir and mbox SHALL answer `hasAttachment` by the rule of the pimdir mark (STORAGE Annex A.1) over the leaf parts of a message. A part SHALL mark the message when its `Content-Disposition` is any but `inline`. A part stating none SHALL mark it unless it is text, multipart or named by a `Content-ID`. An RFC 1847 protocol part SHALL never mark it: any child of a `multipart/encrypted`, any child but the first of a `multipart/signed`. A `Content-Type` that is no pair of RFC 2045 tokens SHALL read as `text/plain`. A `message/rfc822` part is a leaf, the parts of the attached message being its own. The `attachment` search clause SHALL read the same mark on the backends that search locally. JMAP and Graph SHALL relay what the server says; Gmail SHALL leave it `null`.

`attachment list` SHALL stay wider than this mark: it also lists signatures and images named by a `Content-ID`, and some text parts.

#### Scenario: A PDF without a disposition
- WHEN a message holds a single `application/pdf` part with no `Content-Disposition`
- THEN `envelope list --has-attachment` reports `hasAttachment: true` on IMAP, Maildir, m2dir and mbox

#### Scenario: An inline PDF
- WHEN a `multipart/mixed` message holds a text part and an `application/pdf` part with `Content-Disposition: inline`
- THEN `envelope list --has-attachment` reports `hasAttachment: false` on IMAP, Maildir, m2dir and mbox

#### Scenario: A signed message
- WHEN a `multipart/signed` message holds a text part and its signature, marked `attachment` or not
- THEN `envelope list --has-attachment` reports `hasAttachment: false` on IMAP, Maildir, m2dir and mbox

### Requirement: The envelope carries its threading pointers
The shared `Envelope` SHALL carry `message_id` and `in_reply_to`, the RFC 5322 §3.6.4 identity of a message and of the message(s) it replies to, so a client can pair a reply with its parent from a listing rather than by reading bodies.

`in_reply_to` SHALL be a list, the grammar being `1*msg-id`, and every id in it SHALL be normalised exactly as `message_id` is (angle brackets and surrounding whitespace stripped), so the two compare byte-for-byte whatever backend surfaced them.

Each backend SHALL source the field from the response its listing already makes, and SHALL leave it empty rather than issue a request of its own: IMAP from the `ENVELOPE` (RFC 3501 §7.4.2, 9th element), JMAP from the `inReplyTo` property of `Email/get`, Gmail from the metadata headers, Maildir, m2dir and mbox from the parsed message, and pimdir from the stored summary. Graph leaves it empty, `In-Reply-To` living in `internetMessageHeaders`, which a listing selection does not return.

The field SHALL NOT be printed by the `envelope list` text output, where raw msg-ids would be noise; it rides the JSON output.

### Requirement: An IMAP envelope reads its strings as a header
The subject and display names of an IMAP `ENVELOPE`, as `envelope list`, `envelope search`, `imap fetch` and `imap thread` print them, SHALL be read as `message read` reads a header (commands: "Malformed mail reads as its sender meant"): raw 8-bit bytes as UTF-8 when they are valid UTF-8 and as Windows-1252 otherwise, line breaks dropped, adjacent RFC 2047 encoded words of one charset joined as bytes before their charset is decoded, and UTF-7 read as written.

#### Scenario: An emoji split across two encoded words
- GIVEN an IMAP message whose `ENVELOPE` subject is `=?UTF-8?Q?=F0=9F?= =?UTF-8?Q?=98=82?=`
- WHEN `envelope list` prints it
- THEN its subject is 😂, as `message read` shows it

### Requirement: Network backends
IMAP, JMAP, Gmail and Microsoft Graph SHALL each adapt their io-* high-level client. IMAP reuses io-imap's `select`/`fetch`/`store`/`copy`/`move`/`append`/`list`/`status`. JMAP reuses io-jmap's `mailbox_get`/`email_query`/`email_get`/`email_set`/`email_import`/`email_submission_set`/`blob_upload`/`blob_download`, addressing mailboxes by their JMAP id. Gmail treats labels as mailboxes over io-gmail's `labels`/`messages` surface; Graph treats mail folders as mailboxes over io-msgraph's `mail_folders`/`messages` surface.

### Requirement: IMAP move without the MOVE extension
The IMAP adapter's `move_messages` SHALL use `UID MOVE` (RFC 6851) when the server advertises `MOVE`. Otherwise it SHALL `UID COPY` the set to the target, then flag `\Deleted` the source UIDs really copied and, when the server advertises `UIDPLUS` (RFC 4315), `UID EXPUNGE` them. Without `UIDPLUS` it SHALL skip the expunge, leave the messages flagged in the source, and log that at debug. Nothing is flagged or expunged when the copy affected nothing. A plain `EXPUNGE` SHALL NOT be issued, so unrelated `\Deleted` messages stay.

### Requirement: IMAP copy and move counts
`copy_messages` and `move_messages` SHALL count the source UIDs a `COPYUID` reports, a `COPYUID` whose UIDVALIDITY is `0` (forbidden by RFC 4315, sent by Mailo) included. Without a readable `COPYUID` (none sent, as by a server without `UIDPLUS` or a `UIDPLUS` server when nothing matched, or one malformed beyond that UIDVALIDITY) they SHALL count the requested UIDs a `UID SEARCH UID` finds in the source: after the `UID COPY`, which leaves the source untouched, or before a `UID MOVE`, which then moves only those and is not sent when none is present. A tagged `OK` alone SHALL NOT count as a message copied or moved.

### Requirement: IMAP flag counts
The IMAP adapter's `store_flags` SHALL return how many of the given UIDs exist in the mailbox. A `UID STORE` echoing as many messages as UIDs given counts them all; one echoing fewer, which a server may do for a message whose flags did not change, SHALL be followed by a `UID SEARCH UID <set>` whose matches are the count.

### Requirement: Network transport resilience
The network backends SHALL run over a transport that retries a stream reporting it is not ready (`EAGAIN` on Unix, `EINTR`, and the Windows spelling of an expired deadline) rather than ending the exchange on it. Each read and each write carries its own budget of one minute, so a slow but progressing transfer never runs out of it, and exhausting the budget SHALL fail with a message naming it rather than a raw errno.

Opening a connection SHALL arm a socket read deadline matching that budget, so a server going silent on an otherwise healthy connection ends the command instead of blocking forever.

### Requirement: Local storage backends
Maildir, m2dir and pimdir SHALL adapt io-maildir, io-m2dir and io-pimdir. Maildir stores added messages under `cur/` and SHALL read an entry's flags through io-maildir rather than parsing the filename itself, so the meaning of a Maildir name is decided in the library that owns the format. m2dir is content-addressed with no native copy or move, so those are a get plus a store (plus a delete for move), and its flags live in a `.meta/<id>.flags` sidecar. m2dir mailbox `rename` and message `copy`/`move` remain unavailable until io-m2dir supports them. pimdir is an offline cache the sync engine populates, io-pimdir implementing both the store and the engine: reads project the store's items with their typed mail summaries (pimdir STORAGE Annex A) without body reads, and writes are actions queued for the store's owner to apply and a later sync to propagate, never direct SQL.

### Requirement: mbox backend
The mbox backend SHALL adapt io-mbox over the full unified operation set. A mailbox is one mbox file: `mbox.root` is walked recursively, the mailbox `a/b` being the file `a/b` below it (`a.sbd/b` with `mbox.thunderbird`), and `mbox.inbox`, when set, is the spool shown as `INBOX` and marked with the inbox role. An absolute path passed as a mailbox SHALL open that file, so any mbox reads without configuring it. The raw `mbox` command SHALL expose create, rename, delete and list of mbox files, message save, copy and move, and the six flag letters of the `Status` and `X-Status` headers.

### Requirement: An mbox message is its content id
The mbox backend SHALL address a message by its io-mbox content id, a hash of the message with the flag and metadata fields left out, so a flag write by Himalaya or any other client keeps it. An id the index no longer holds SHALL fail naming the mailbox. An append SHALL report the id the synced index gives the new message, numbered like any duplicate.

### Requirement: mbox flags interoperate
Shared flags SHALL map onto `Status: R` (seen), `X-Status: A`, `F`, `T` and `D` (answered, flagged, draft, deleted), any other flag going to `X-Keywords`. A message marked seen SHALL be marked old (`O`) too, as mutt writes it, since GNU mail lists a `Status: R` alone as new. A unified `flag set` SHALL keep the `O` a message carries, no shared flag naming it.

### Requirement: mbox writes lock and rewrite in place
Every mbox write SHALL go through io-mbox under the dotlock and the fcntl lock, each one skippable through `mbox.lock.dotlock` and `mbox.lock.fcntl`, waiting `mbox.lock.timeout` seconds for a busy one. Flag changes and removals SHALL rewrite the file in place, the whole set of messages a command names in one rewrite. A move SHALL copy then remove, so an interruption leaves the messages in both files.

### Requirement: mbox reads go through a cache
The mbox backend SHALL keep, per file, the io-mbox scan index and the envelope of every message already parsed, under `<XDG cache>/himalaya/mbox/`. Every operation SHALL sync the index first, so an unchanged file is not scanned and a delivery is scanned alone, and a listing SHALL parse only the messages the cache lacks. The cache SHALL be disposable: a missing, stale or unreadable one is rebuilt, failing to write one is not an error, and a read finding its entry stale SHALL rebuild the index and retry once.

### Requirement: A Maildir is named relative to the root
A Maildir mailbox SHALL be named, in `mailbox list` and `maildir list`, by its path relative to `maildir.root` (`[Gmail]/Archive`), the name `-m` takes, so nested Maildirs sharing a last segment stay apart. The root itself keeps its folder name.

### Requirement: Maildir surfaces custom keywords on demand
The Maildir backend SHALL surface custom (non-IANA) keywords on read when told which convention the mailbox uses, so a keyword written by dovecot, mbsync, OfflineIMAP, mutt or notmuch matches a `flag <name>` search as it does on the network backends. `maildir.keywords.dovecot` SHALL resolve the lowercase info-section slot letters through the resolved mailbox's own `dovecot-keywords` file, and `maildir.keywords.header` SHALL read keywords from `X-Keywords` (comma-separated) or `X-Label` (space-separated). Both default to off, and with both off the flag set SHALL be exactly the six standard info-section letters as before.

Keyword reading is not a round trip: no command can name a custom keyword, so a `FlagOp::Set` store SHALL replace the whole set and drop any keyword the message carried.

A sidecar that is absent, unreadable or disabled SHALL yield no keywords rather than fail the listing, since a mailbox without one is the normal case rather than an error.

### Requirement: A Message-ID is not an address
The pimdir backend SHALL NOT assume an item's link id is the `Message-ID` its body carries, nor that a `Message-ID` identifies at most one message in a mailbox. A store may hold two messages of one mailbox sharing a `Message-ID`, keyed apart by the store (pimdir STORAGE §9), and both SHALL list, read and act as ordinary messages, each with its own public id and neither marked.

What stays unique is the key and the public id: `(collection, link_id)` still names one item and `seq` still names one message. What ends is the link id being derivable from the body, so a read that re-derives an identity in order to address a row is addressing an unknown number of them.

A mailbox holding one `Message-ID` twice is ordinary (a double delivery, a retried append, a restore, a copy of a sent message), and the store now keeps both rather than one. Showing one of them, or resolving an identity to whichever row came first, hides a message the server holds.

### Requirement: pimdir shows a short public id
The pimdir backend SHALL show and accept each message's public id (`items.seq`, a small store-assigned integer, the same across every mailbox the message is filed in) as its `Envelope.id`, not the internal `link_id`. It SHALL check the id against the collection before reading a body or staging an action, and SHALL fail clearly on a non-numeric or unknown one. `add_message` SHALL return the link id it staged: a queued create has no `seq` yet, the store assigning one when its owner applies the action.

Addressing by the public id is what keeps two duplicated messages distinguishable: they carry one `Message-ID` between them and have two `seq`s, so an address derived from the body would be ambiguous where a `seq` is not.

### Requirement: A pimdir mailbox is its collection id
The pimdir backend SHALL show and accept a mailbox as the store's collection id, verbatim: the collection `imap/INBOX` is the mailbox `imap/INBOX`. It SHALL NOT derive, strip or accept a shortened spelling, and no configuration SHALL offer one.

The id is opaque to the store, which neither parses nor validates it (pimdir STORAGE §9.2) and models hierarchy through `parent` rather than through a separator. Any shortening is therefore a guess at the producer's convention, and one that makes a single mailbox answer to two spellings. This is the JMAP backend's shape, whose ids are opaque server strings, and `[mailbox.alias]` is the shortcut for both.

`Mailbox.name` SHALL carry the collection row's own name rather than one derived from the id.

A mailbox is a collection declared `message/rfc822`, matched on the bare media type. A collection declaring no kind is not a mailbox: the store cannot summarise it, and the draft that left kinds undeclared is refused on open rather than read.

A value matching no collection id of the account SHALL be taken as a role name when it is one (`inbox`, `sent`, `drafts`, `trash`, ...), addressing the account's mailbox whose recorded role (`collections.role`) it is; several mailboxes carrying the role SHALL be refused naming them. A collection's name SHALL NOT address it: two sources may hold mailboxes of one name. A value matching neither SHALL be refused naming the mailboxes the account holds. It SHALL NOT be passed to the store unresolved, which reads as a mailbox that exists and is empty.

#### Scenario: The inbox by its role
- GIVEN a pimdir account whose store records the role `inbox` on `imap/INBOX`
- WHEN `envelope list` runs without `-m`
- THEN the envelopes of `imap/INBOX` are listed

#### Scenario: The trash by its role
- GIVEN a pimdir account with no `mailbox.alias.trash` and a mailbox recorded with the role `trash`
- WHEN `message delete` runs on a message of another mailbox
- THEN the message is staged as a move to that mailbox

### Requirement: pimdir reads one account
The pimdir backend SHALL show the collections of one account (pimdir STORAGE §9.2), `pimdir.account` naming it. Unset, it is derived: a store holding one account, or one ungrouped set, is read as that one, and a store holding several is refused naming them rather than guessing one and showing the wrong mailbox set.

### Requirement: pimdir is a reader and a producer, never the owner
The pimdir backend SHALL treat the store as a possibly-partial cache owned by the sync engine. `get_message` on an item whose body is not local (`level < Full`, no stored object, or a stored object whose blob file is gone, pimdir GUIDE §14) SHALL fail with the code `body-pending` (commands, "A JSON error carries a stable code"), the cue to sync, not a data-loss error; the item still lists.

Reads SHALL go through `PimdirReader`, the role that takes no lock (pimdir STORAGE §8) and carries no write at all, so a sync in flight neither blocks Himalaya nor is blocked by it, and the backend cannot drain the queue or sweep the store even by mistake.

The store's search index (index.db) SHALL be opened read-only, never created, rebuilt nor refreshed: the sync engine owning the store builds and refreshes it (search, "pimdir searches through the store's index").

An envelope SHALL be built from the item's typed mail summary and the addresses joined to it (pimdir STORAGE Annex A.1, A.6): every `From` and `To` address with its display name, the `In-Reply-To` list, the date, the size and the attachment mark as stored, with no body read. `hasAttachment` SHALL be `true` or `false` as the store marks it, and `null` on a row that holds no mark. A listing SHALL take the store's own order, newest first, the mail sort key being the date.

The reader SHALL overlay the queue (pimdir STORAGE §15.4), so an action this client staged is visible on the next read rather than on the next sync: a staged `set-flags`, `update`, `remove`, `move` or `copy` changes what a listing shows. Each addresses a message that already exists and keeps its public id, so a staged write never changes how a message is addressed.

A write SHALL be staged as a queued `PimdirAction` through a producer handle (`store_flags`→`SetFlags`, `add_message`→`Add`, `copy_messages`→`Copy`, `move_messages`→`Move`, `delete_messages`→`Remove`), addressed by the public `seq`, for the store's owner to apply and push. The backend SHALL NOT write the index, load a collection, or run the owner's object sweep: a sweep run beside a sync destroys the bodies it has streamed but not yet attached, which STORAGE §14 explicitly invites it to leave pending. A body an action references SHALL be written to the blob store in the transaction appending the action, the queue row being what pins it.

`SetFlags` carries the whole replacement set, so applying it twice lands the same state. An add or a remove on a set the store reports as unknown SHALL be refused, saying the message's flags were never synced: it has no markers to build on, and the replacement would erase whatever the server holds. A `set` replaces the whole set, so it SHALL stage one regardless. A change whose replacement set equals the set the message shows SHALL stage nothing for it, the message still counting as found. pimdir has no native trash.

An added message SHALL derive its link id through io-pimdir's mail derivation (`summary::mail::derive`), the one implementation of STORAGE Annex A.1, which is the bare `Message-ID` with nothing prepended, and SHALL name it on the queued `Add`. The action carries no summary: the owner derives the summary and the sort key from the body when it applies the action, through the same call, so the two never disagree. An `Add` whose link id the mailbox already holds live, or has an `add` queued under, SHALL be refused with the code `item-duplicate` through io-pimdir's `enqueue_with_body_checked`, and nothing SHALL be queued nor written: the owner would park such a row (pimdir STORAGE §15.3) rather than deduplicate against the stored copy or mint a second key. Minting is the store's answer to what a source hands over; refusing is its answer to a producer authoring a message the collection already has. An `add` still parks when the source lists the same `Message-ID` after it was staged.

#### Scenario: An add on flags never synced is refused
- GIVEN a pimdir message whose flags the sync engine never read
- WHEN `himalaya flag add -m imap/INBOX -f flagged 42` runs
- THEN the command fails saying the flags of `42` were never synced, and nothing is queued

#### Scenario: A flag the message carries already
- GIVEN a pimdir message showing `\Seen`
- WHEN `himalaya flag add -m imap/INBOX -f seen 6` runs
- THEN no row is queued, and the command reports one message found

#### Scenario: A body purged under a held item
- GIVEN a pimdir item whose stored object's blob file is gone
- WHEN `message read` runs on it
- THEN the command fails with the code `body-pending`

### Requirement: A pimdir body is written under the staging lock
A pimdir write carrying a body (`add_message`, `send_message`) SHALL hand the body to its producer with the action in one step (`enqueue_with_body_checked`): the capability gate and, for an add, the duplicate check run on the bytes in hand, then the body is written inside the producer's staging window (pimdir STORAGE §8) and the row pinning it appended, in one transaction. A write refused by the gate or the duplicate check SHALL leave no body in the blob tree. No body SHALL reach the blob tree while a collector holds the lock: the write waits for it.

#### Scenario: A collector running beside a staged add
- GIVEN a pimdir store whose collector holds `objects.lock` exclusively
- WHEN a message is added to a mailbox
- THEN no body is written until the collector releases the lock, and then the body lands and its `add` is queued

#### Scenario: A refused add
- GIVEN a pimdir mailbox holding a message whose `Message-ID` is `d1@x.org`
- WHEN another body with that `Message-ID` is added to it
- THEN the add fails with `item-duplicate` and its body is not in the blob tree

### Requirement: A pimdir store failure carries its code
A failure io-pimdir reports SHALL keep io-pimdir's message as a cause and carry, under `--json`, the code `PimdirError::code()` gives it, spelled the same, when it is one a caller acts on: `store-version`, `store-uncreated`, `store-foreign`, `store-corrupt`, `store-not-current`, `store-hash-algo`, `sync-version`, `store-busy`, `store-owned`, `store-staging`, `store-in-flight`, `store-exposed`, `item-duplicate`, `flag-invalid`, `action-malformed`, `conflict-unread`, `unsupported`, `no-performer` and `ambiguous-performer`. An SQL, I/O or JSON failure carries none. Himalaya SHALL NOT derive a code from a message, and SHALL NOT advise deleting the store.

A store of another draft of the format (pimdir STORAGE §4.2), earlier or later, or one naming no format version, is `store-version`: drafts are exact, so it opens under no other draft and nothing reconciles or migrates it. Its message SHALL say to upgrade Himalaya when the store's sync engine is newer, else to reinstall the store as neverest's MIGRATION.md says, which syncs out the writes still queued in it before moving it aside. A store whose directory or pimdir.db another user can read or write, or a link where a lock file belongs, is `store-exposed`, refused before anything in it is read, its message saying to restrict it.

#### Scenario: A store another draft wrote
- GIVEN a pimdir store whose `store_meta.format_version` names another draft than Himalaya's io-pimdir follows
- WHEN any pimdir command opens it
- THEN the command fails with the code `store-version`, naming the draft, saying to upgrade Himalaya or reinstall the store as neverest's MIGRATION.md says, and never to delete it

#### Scenario: A store others can read
- GIVEN a pimdir store whose directory is mode `0755`
- WHEN any pimdir command opens it
- THEN the command fails with the code `store-exposed`, saying to restrict it

#### Scenario: A draft the mailbox holds already
- GIVEN a pimdir mailbox holding a message whose `Message-ID` is `d1@x.org`
- WHEN `himalaya message add -m imap/Drafts` stages a message with that `Message-ID`
- THEN the command fails with the code `item-duplicate`, and nothing is queued

### Requirement: A queued creation is reported, not listed
A queued creation has no public id until the store's owner applies it, so the pimdir backend SHALL NOT project one as an envelope, and SHALL NOT put a placeholder in `Envelope.id`. `add_message` returns the link id it staged, which identifies the creation across the window. `himalaya pimdir queue list` is where queued creations show.

### Requirement: The pimdir subcommand reads and retracts the queue
Himalaya SHALL carry a `pimdir` subcommand for what the operator CLI cannot do without knowing mail. `queue list` SHALL render a queued creation as a message (flags, subject, recipient, and when it was queued, from the row's `created_at`) where the kind-agnostic `pimdir` binary can only print ids and hashes. The queued action carries no summary, so the row SHALL be derived from the body the action pins, through the same derivation the owner applies; an action pinning no body renders with its flags alone.

Under `--json` the message SHALL be printed as an `envelope` with the shared envelope's keys and `"id": null`, a queued message having no id until the owner applies its action. No placeholder SHALL stand in for it (no `""`, `0` nor row id): the row id is the row's own `id`, the title of its record.

`queue cancel` SHALL retract one pending or parked row of the account's queue, confirming first unless `--yes`, and SHALL name the row's action kind in its prompt and output. A row anchored on a collection of another account, or one neither pending nor parked, SHALL be refused before anything is opened, and stays as it was.

Taking the owner role briefly is what cancelling costs (pimdir STORAGE §15.5); the backend read and write paths SHALL NOT reach it. The cancel SHALL open the store only as it stands (io-pimdir `open_current`): it SHALL NOT create, migrate nor reconcile the schema. A store of another draft SHALL be refused as `store-version` and a damaged one as `store-corrupt`, each left as it is. A store another process owns SHALL be refused immediately, saying a sync is running and that the action may already have been applied.

#### Scenario: A queued creation is rendered from its body
- GIVEN a queued `add` pinning a body whose headers carry a subject and a recipient
- WHEN the queue is listed
- THEN the row shows that subject and recipient, and its envelope's `id` is `null` under `--json`

#### Scenario: A cancel on a store of another draft
- GIVEN a store whose `store_meta.format_version` names another draft
- WHEN `himalaya pimdir queue cancel` runs on one of its rows
- THEN the command fails with the code `store-version`, pointing at neverest's MIGRATION.md, and neither the store's stamp nor the row changes

#### Scenario: Another account's row
- GIVEN a store shared by two accounts and a row queued on the other account's collection
- WHEN `himalaya pimdir queue cancel` runs on that row
- THEN the command fails saying the row is not in the account's queue, and the row stays queued

### Requirement: pimdir writes name their queue row
`pimdir message add` and `pimdir message send` SHALL stage a message as the unified `message add` and `message send` do, and print the queue row it waits in with the message's bare `Message-ID`: `{"row": i64, "messageId": string}` for an add, plus `"copy": bool` (the `submit` row carries the copy) and `"copyRow": i64` (a copy queued beside the send, when its performer declares no capabilities) for a send. `pimdir flag add|set|remove` SHALL print the row of each message they found: `{"messages": [{"id": string, "row": i64 | null}]}`, `null` for a message the change left as it was, for which nothing was staged. The unified commands SHALL keep printing no queue detail. `pimdir queue list` and `queue cancel` SHALL name a row `row` as well.

`pimdir queue show <ROW>` SHALL say where a row of the account's queue stands, read through io-pimdir's `action_status` (pimdir STORAGE §15.4): `{"row", "state": "pending" | "parked" | "applied" | "unknown", "collection", "kind", "attempts", "error", "appliedAt", "seq"}`, a key the state does not carry being `null`. A pending or parked row carries `collection`, `kind` and `attempts`, a parked one `error` too; an applied one carries `collection`, `appliedAt` and, for an add, `seq`, the public id of the message it created. A row neither queued nor in the store's receipts (kept seven days), or one anchored on another account's collection, SHALL read as `unknown`: cancelled, or applied long ago. A row the sync engine performed rather than applied, a message sent or a mailbox created, renamed or deleted, reads as `applied` without `seq`, its receipt recorded on acknowledgement (pimdir STORAGE §15.5). A receipt lives under the collection its row was queued on and follows it: a renamed mailbox's row reads `applied` under the mailbox's new id, and a deleted mailbox's row reads `unknown` once the deletion leaves the store, the receipt going with the collection, the mailbox missing from `pimdir mailbox list` being what says it was deleted.

The plain output of every `pimdir` subcommand SHALL end with a newline.

#### Scenario: A staged draft is followed by its row
- GIVEN a pimdir account
- WHEN `himalaya --json pimdir message add -m imap/Drafts` stages a message
- THEN it prints `{"row":12,"messageId":"d1@x.org"}`, and `pimdir queue show 12` prints `"state":"pending"` until the sync engine applies the row, then `"state":"applied"` with the `seq` the message is read by

### Requirement: A pimdir refile or delete stages the ids it found
`copy_messages`, `move_messages` and `delete_messages` SHALL check every given id against the mailbox before staging the first row, skip an id the mailbox does not hold, stage one row per id found and report how many they found (commands, "A command given message ids says how many it found"). A call finding none SHALL stage nothing. A malformed id SHALL fail the call before anything is staged.

#### Scenario: One id of two exists
- GIVEN a pimdir mailbox holding message 3 and no message 999
- WHEN `message copy --from imap/INBOX --to imap/Trash 3 999` runs
- THEN one `copy` row is queued for 3, and the command succeeds reporting one message

### Requirement: The pimdir queue view shows parked rows
`himalaya pimdir queue list` SHALL list the mailbox's parked rows, any action kind, beside its queued creations and sends: the row id, the action kind, when it was queued, and the error that parked it, as `parked` under `--json`. `queue cancel` SHALL take a parked row as it takes a pending one.

#### Scenario: A duplicate draft parks
- GIVEN a staged `add` the sync engine parked because the source listed the same `Message-ID` before applying it
- WHEN `himalaya pimdir queue list -m imap/Drafts` runs
- THEN the row is listed as parked with its error, and `pimdir queue cancel` on its id removes it

### Requirement: pimdir creates a mailbox by queueing an intent
`pimdir mailbox create <NAME> [--parent <MAILBOX>]` SHALL enqueue one `collection-create` intent (pimdir STORAGE Annex B.2) with the `v: 1` payload `{"v": 1, "source": id, "name": text, "parent": collection?}`, and print `{"row": i64, "name": text, "parent": collection | null, "source": id}`. It SHALL NOT create a collection in the store: the mailbox arrives with the sync that performs the intent.

The row SHALL be anchored on `parent`, which must be a mailbox of the account, else on the account's first mailbox by id, and queued through io-pimdir's `enqueue_collection_create`. The performer SHALL be named as for a send (capability `collection.create`): several candidates and no recorded choice are refused, naming them. A store whose sources declare no capabilities SHALL be refused, its owner predating the intent. An empty name, or one holding a control character, SHALL be refused before anything is staged. A name the server already holds is the server's to refuse: the row parks, and `pimdir queue show` says why.

#### Scenario: A folder is created through the sync engine
- GIVEN a pimdir account whose `imap` source declares `collection.create`
- WHEN `himalaya --json pimdir mailbox create Projets` runs
- THEN one `collection-create` row is queued with `{"v":1,"source":"imap","name":"Projets"}`, and the command prints its `row` and `"source":"imap"`

### Requirement: pimdir renames or deletes a mailbox by queueing an intent
`pimdir mailbox rename <MAILBOX> <NAME>` SHALL enqueue one `collection-rename` intent (pimdir STORAGE Annex B.2) with the `v: 1` payload `{"v": 1, "source": id, "name": text}` and print `{"row": i64, "mailbox": collection, "name": text, "source": id}`. `pimdir mailbox delete <MAILBOX>` SHALL enqueue one `collection-delete` intent with `{"v": 1, "source": id}` and print `{"row": i64, "mailbox": collection, "source": id}`. Neither SHALL rename or delete a collection in the store: the change arrives with the sync that performs the intent.

The row SHALL be anchored on the mailbox, which must be a mailbox of the account (an id or a role, after the account's aliases), the payload never restating it, and queued through io-pimdir's `enqueue_collection_rename` or `enqueue_collection_delete`. The performer SHALL be named as for a creation (capability `collection.rename` or `collection.delete`, each gating its own intent): several candidates and no recorded choice are refused, naming them, and a source declaring the capability `none` on that mailbox (an IMAP `INBOX`, a system folder) refuses it. A store whose sources declare no capabilities SHALL be refused. A rename name empty or holding a control character SHALL be refused before anything is staged.

A rename keeps the mailbox's parent, `name` being one level of the hierarchy. The store records no hierarchy separator, so a name holding the server's, or one a sibling already holds, is the performer's to refuse: the row parks, and `pimdir queue show` says why. A delete removes the mailbox and the messages it holds on the server, and never another mailbox: the store records no mailbox's parent, so a mailbox holding others is the performer's to refuse, the row parking naming them.

#### Scenario: A folder is renamed through the sync engine
- GIVEN a pimdir account whose `imap` source declares `collection.rename`
- WHEN `himalaya --json pimdir mailbox rename imap/Projets Archives` runs
- THEN one `collection-rename` row is queued on `imap/Projets` with `{"v":1,"source":"imap","name":"Archives"}`, and the command prints its `row`, `"mailbox":"imap/Projets"` and `"source":"imap"`

#### Scenario: A folder holding another is not deleted
- GIVEN a pimdir account whose `imap` source declares `collection.delete`, and a server mailbox `Projets` holding `Projets/2025`
- WHEN `himalaya pimdir mailbox delete imap/Projets` runs and the sync engine performs the row
- THEN the server deletes nothing, the row parks naming `Projets/2025`, and `pimdir queue show` prints it as `parked` with that error

### Requirement: pimdir stages flags by their raw spelling
`himalaya pimdir flag list|add|set|remove` SHALL read and stage the flags of a pimdir message by their raw spelling, verbatim. A value naming an IANA flag SHALL be accepted only in its canonical spelling, compared without case and staged in the canonical case, a bare `seen` or `$seen` being refused naming `\Seen`; `\Deleted` and `\Recent` SHALL be refused. A backslash names a system flag, so one in front of any other word SHALL be refused, as SHALL a character IMAP forbids in a flag (`(`, `)`, `{`, `%`, `*`, `"`, `]`, `\`). A value the store cannot hold (pimdir STORAGE §13), empty or holding whitespace, a control character or a character outside ASCII, SHALL be refused with the code `flag-invalid`, the code io-pimdir's enqueue gives a flag set holding one, and nothing staged. Any other value is a keyword.

`add`, `set` and `remove` SHALL fail when no message is named or none of the named ones exists, and SHALL stage one `SetFlags` per message through the backend's `stage_flags`, checked against the sources' declarations (pimdir STORAGE §15.6): a source declaring `mail.flags.keywords` as `none` SHALL refuse a keyword change with its detail, and one declaring it in part SHALL queue it with the declaration's warning as it is. A change on `$Important` or `$Junk` SHALL be gated by `mail.flags.important` or `mail.flags.junk` from a source stating that row, `none` included, and by `mail.flags.keywords` from one stating none (pimdir STORAGE Annex B.1), io-pimdir's gate deciding which. `list` SHALL print the flags a message carries as the overlaying reader shows them, and `null` under `--json` for a message whose flags the sync engine never read.

An add or a remove SHALL match the flags a message carries as flags, not as spellings: a marker the store holds as `\seen`, or as Thunderbird's `Junk`, is the one `\Seen` or `$Junk` keeps on an add and drops on a remove. This holds for the shared flag commands on a pimdir account, which stage through the same path.

#### Scenario: A keyword is staged and read back
- GIVEN a pimdir account whose `imap` source declares `mail.flags.keywords`
- WHEN `himalaya --json pimdir flag add -m imap/INBOX -f act 42` runs
- THEN it prints `{"messages":[{"id":"42","row":7}]}`, and `pimdir flag list -m imap/INBOX 42` lists `act` before the next sync

#### Scenario: A source without keywords refuses one
- GIVEN a pimdir account whose `msgraph` source declares `mail.flags.keywords` as `none`
- WHEN `himalaya pimdir flag add -m msgraph/Inbox -f act 42` runs
- THEN the command fails naming `mail.flags.keywords`, the `msgraph` source and its detail, and nothing is queued

#### Scenario: A flag held in another spelling is removed
- GIVEN a pimdir message carrying `\seen` and `Junk`
- WHEN `himalaya pimdir flag remove -m imap/INBOX -f '\Seen' -f '$Junk' 42` runs
- THEN the staged set holds neither

#### Scenario: A bare IANA name is refused
- WHEN `himalaya pimdir flag add -m imap/INBOX -f seen 42` runs
- THEN the command fails naming `\Seen`, and nothing is queued

#### Scenario: A flag the store cannot hold
- GIVEN any pimdir account
- WHEN `himalaya --json pimdir flag add -m imap/INBOX -f 'to do' 42` runs
- THEN the command exits 1 with the code `flag-invalid`, and nothing is queued

### Requirement: pimdir stages a flag change on every message or none
`stage_flags`, which the shared `store_flags` goes through, SHALL check every message of a call the mailbox holds against the sources' declarations before it queues the first row, so a call the check refuses on any message queues nothing. An id the mailbox does not hold SHALL be skipped, the call staging the others and reporting how many it found (commands, "A command given message ids says how many it found"); a malformed id SHALL fail the call, as it does on IMAP. Each row is still its own transaction: an enqueue failing after the check leaves the rows queued before it. This holds for the unified `flag add|set|remove` on a pimdir account as for `pimdir flag`.

#### Scenario: A refusal part-way stages nothing
- GIVEN a pimdir account whose `msgraph` source declares `mail.flags.keywords` as `none`, and message 41 of `msgraph/Inbox` carries the keyword `act`
- WHEN `himalaya flag set -m msgraph/Inbox -f seen 40 41 42` runs, the set dropping `act` from 41 only
- THEN the command fails naming `mail.flags.keywords` for message 41, and no row is queued for 40, 41 or 42

### Requirement: pimdir lists each mailbox with its coverage
`pimdir mailbox list` SHALL list the account's mailboxes sorted by id, each as `{"id", "name", "role", "total", "unread", "coverage", "round"}`: `role` as `mailbox list` shows it (`null` when none), `total` and `unread` counted in the store, `coverage` the collection's coverage (pimdir STORAGE §14.1, the narrowest of its sources') as `{"since", "until", "at"}`, and `round` a round a source has under way as `{"since", "until", "startedAt"}`. Bounds are RFC 3339 instants, `null` for an open end. `coverage` SHALL be `null` until every source of the collection closed a round on it, and on a store its owner has not reconciled; `round` SHALL be `null` when no round is under way. The plain list SHALL show the coverage and the round as their days (`since 2026-09-07`, `all`). The unified `mailbox list` SHALL carry none of it.

#### Scenario: A mailbox synced from a date
- GIVEN a pimdir account whose `imap` source closed a round on `imap/INBOX` over the mail since 2026-09-07
- WHEN `himalaya --json pimdir mailbox list` runs
- THEN `imap/INBOX` carries `"coverage":{"since":"2026-09-07T00:00:00Z","until":null,"at":…}`, and a mailbox no round closed on carries `"coverage":null`

### Requirement: pimdir reads only the page an envelope listing needs
`envelope list` on a pimdir account with a page size SHALL read the mailbox newest first by keyset pages and stop at the end of the page asked for: page N of size S reads at most N × S items, overlaid as every read is. Without a page size it SHALL read the whole mailbox.

`envelope search` SHALL read newest first and, when the order is the default one (date descending, or a sort that is `date desc` alone), stop once the page asked for is full. Date clauses at the top of the query (`date`, `after`, `not after`, joined by `and`) SHALL bound the walk on the store's sort key, the mail sort key being the summary date (pimdir STORAGE §9.3, Annex A.1). `not flag seen` at the top of the query SHALL read through io-pimdir's unread chip (`list_mail_page_filtered`) when no queued `set-flags`, `update`, `remove`, `move` or `copy` touches the mailbox, the chip reading committed rows only. Every clause SHALL still be evaluated on the envelopes read, and every other sort applied to them, so a search returns the same envelopes in the same order as a whole-mailbox read. A row read whose sort key is not its summary date in the store's canonical form SHALL make the search read the whole mailbox instead.

#### Scenario: The first page of a large mailbox
- GIVEN a pimdir mailbox holding 50,000 mails
- WHEN `himalaya envelope list -s 50` runs
- THEN the store reads the 50 newest items, and one more per queued removal, rather than the 50,000

### Requirement: pimdir lists the account's attachments
`pimdir attachment list` SHALL list the files the account's messages attach, read from the stand-ins of the account's attachments collection (pimdir STORAGE §14.3) through io-pimdir's `list_attachments_by_account`, newest message first, opening no body. It SHALL print `{"attachments": [...], "nextPage"}`, each file as `{"id", "name", "mediaType", "size", "section", "messageId", "mailbox", "sender", "senderName", "date"}`: `id` the stand-in's public id; `name` its decoded name, `null` when the part names none; `mediaType` lowercased without parameters; `size` in octets once the transfer encoding is undone; `section` its IMAP section in the message (RFC 3501 §6.4.5); `messageId` and `mailbox` the public id and collection of one live placement of the message attaching it, as `envelope list` and `message read` take them; `sender` and `senderName` the message's sender; `date` its date, RFC 3339. Ids SHALL be strings.

A page SHALL hold `--page-size` files (else `envelope.list.page-size`, else 25). `nextPage` SHALL be an opaque token that `--page-token` takes to start after the page's last file, and `null` exactly when no file follows. Pages SHALL follow one another by keyset on the message's sort key and the stand-in's id, so a write between two pages neither repeats nor skips a file already listed. A token this command did not print SHALL be refused. An account with no attachments collection, or a store without stand-ins, SHALL list none rather than fail.

`pimdir attachment search <WORDS>...` SHALL print the same page, keeping the files for which every word appears, ignoring case (Unicode lowercase; accents count), in the file name, the sender's address or the sender's name. It SHALL read the listing newest first and stop once the page and one more file are found. A query of no word SHALL be refused.

#### Scenario: A page of files and the next one
- GIVEN a pimdir account whose stored messages attach four files
- WHEN `himalaya --json pimdir attachment list -s 3` runs
- THEN it prints the three files of the newest messages and a `nextPage` token, and `pimdir attachment list -s 3 --page-token <TOKEN>` prints the fourth with `"nextPage":null`

#### Scenario: Invoices from one sender
- GIVEN a pimdir account where `billing@shop.test` sent `Facture-Octobre.pdf` and another sender `plage.jpg`
- WHEN `himalaya --json pimdir attachment search facture billing` runs
- THEN it prints `Facture-Octobre.pdf` only

### Requirement: pimdir sends by queueing a submit intent
The pimdir backend SHALL send a message by staging its raw bytes and enqueueing one `submit` action, which whatever owns the store performs through its own channel. It SHALL NOT open a network connection to send, and SHALL NOT use the account's `smtp` section.

The payload SHALL be built by io-pimdir's `PimdirSubmit` (STORAGE Annex B.2), `v: 1` JSON carrying `source`, `from`, `rcpts`, `subject`, and `copy` when the intent carries the copy, the keys and values a performer reads back with `PimdirSubmit::from_payload`. The message is the row's body (`queue.object_hash`), never a member of the payload. `source` SHALL name the performer: the declared one, resolved as `pimdir performer` shows it, else, when the sources syncing the anchor declare nothing, the one source syncing it. An anchor no source syncs yet SHALL be refused with `no-performer`, and one several undeclared sources sync with `ambiguous-performer`, nothing staged. The envelope is derived from the headers by the same function the SMTP adapter uses: the first `From:` address, every `To:`, `Cc:` and `Bcc:` address, and the decoded subject. A message with no `From:` or no recipient SHALL be refused before anything is staged. The body SHALL be stored as given, `Bcc:` included; removing it is the sending channel's job.

The row SHALL anchor on the `--save` mailbox when one is given, else on the mailbox `mailbox.alias.sent` names; with neither, the send SHALL be refused, naming the alias to set, rather than create a collection.

The command SHALL NOT report the queue row id in its output; the pimdir backend SHALL log it at info level.

#### Scenario: A send made offline waits in the queue
- GIVEN a pimdir account with `mailbox.alias.sent` set and no network
- WHEN a message is sent
- THEN one `submit` row is queued on the sent mailbox with the message's envelope

#### Scenario: A Bcc recipient is in the envelope
- GIVEN a message with `To: a@x.org` and `Bcc: b@x.org`
- WHEN it is sent on a pimdir account
- THEN the payload's `rcpts` holds both addresses and the stored body still carries `Bcc:`

#### Scenario: No anchor is refused
- GIVEN a pimdir account with no `mailbox.alias.sent`
- WHEN a message is sent without `--save`
- THEN nothing is staged and the error names `mailbox.alias.sent`

#### Scenario: A mailbox no source syncs yet
- GIVEN a pimdir mailbox the sync engine declared and no source syncs yet
- WHEN a message is sent anchored on it
- THEN the command fails with the code `no-performer`, and nothing is queued

### Requirement: The queue view shows queued sends
`himalaya pimdir queue list` SHALL render a queued `submit` as a message, derived from the body it pins as a create is, marked as a send, beside the queued creates of the same mailbox.

### Requirement: Append gap
Graph SHALL NOT implement `add_message`: a MIME message it creates stays a draft, so it declares `message.add` `none`, and a `message add`, or a save asked of a composer or of `message send`, is refused before connecting. Gmail SHALL implement it through `messages.insert`, the mailbox and the flags becoming labels and the `Date:` header dating the message, without sending it or running filters. Every backend implements `search_envelopes` (see the search capability).

The shared `add_message` result SHALL carry an optional id, absent only when an IMAP or JMAP server acknowledges the write without reporting one. An IMAP `APPEND` acknowledged by the server SHALL stay a success when neither `APPENDUID` (RFC 4315) nor the fallback `Message-ID` search yields a UID, and a requested send SHALL go on. An `APPEND` rejected by the server SHALL stay an error.

### Requirement: Sending transport
Backends that self-send (JMAP, Gmail, Graph) SHALL route `send_message` through their own API, and pimdir through its queue. Storage backends that cannot send (IMAP, Maildir, m2dir, mbox) SHALL send through the account's SMTP transport, adapted in `src/smtp/backend.rs` over io-smtp, which parses the RFC 5321 envelope from the raw message headers. The transmitted message SHALL NOT carry the `Bcc:` field (RFC 5322 3.6.3), which io-smtp removes after the envelope is derived. The protocol-level `smtp send` SHALL transmit its message verbatim, its envelope being explicit. Its `--notify`, `--ret` and `--envid` SHALL request delivery status notifications (RFC 3461), `NOTIFY` going on every `RCPT TO` and `RET` and `ENVID` on `MAIL FROM`; the command SHALL fail before `MAIL FROM` when the server does not announce `DSN`, and SHALL refuse `never` combined with another condition and an `ENVID` needing xtext encoding or longer than 100 characters. The unified `message send` requests none.

### Requirement: A JMAP send files the message as sent
A JMAP send SHALL stage the message in the drafts mailbox with `$draft`, then submit it with an `onSuccessUpdateEmail` patch (RFC 8621 section 7.5) moving it to the sent mailbox, unsetting `$draft` and setting `$seen`. The drafts mailbox SHALL be `jmap.drafts-mailbox-id`, else the `drafts`-role one, the send failing without either; the sent mailbox SHALL be `jmap.sent-mailbox-id`, else the `sent`-role one, the message only losing `$draft` without either.

### Requirement: Mailbox role
A shared mailbox SHALL carry an optional role (inbox, all, archive, drafts, flagged, important, junk, sent, subscribed, trash, or a verbatim unknown one), shown by `mailbox list` in its text and JSON output. JMAP reads it from the mailbox `role`, Gmail from its fixed system-label ids, Microsoft Graph from its well-known folder names resolved to folder ids in one `$batch` request, pimdir from the role its sync engine recorded from the server (`collections.role`), and IMAP marks only `INBOX`, as mbox marks its spool when one is configured. Maildir and m2dir have no native role. On JMAP, Gmail, Graph and pimdir a role name addresses the mailbox carrying it. A `mailbox.alias.<role>` entry overrides the native role of the mailbox it names, and `message delete` takes its trash from that alias before the backend's trash role.

### Requirement: Every backend builds alone
`cargo clippy --all-targets -- -D warnings` SHALL pass with the default features; with `--all-features`; with `--no-default-features` and a TLS provider alone (`rustls-ring`); with a TLS provider and each backend feature alone (`imap`, `smtp`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox`, `m2dir`, `sieve`, `pimdir`); with `native-tls,jmap`; and with the four local backends together (`maildir,mbox,m2dir,pimdir`). Code only some backends use SHALL be gated on the item with exactly those features, never allowed crate-wide. A configuration type only some backends read SHALL stay compiled in every build and be allowed to go unread in the builds without them, so every build parses and checks the same configuration, the blocks of the backends it leaves out included.

#### Scenario: A Maildir-only build
- GIVEN `--no-default-features --features rustls-ring,maildir`
- WHEN `cargo clippy --all-targets -- -D warnings` runs
- THEN it passes, where it reported the proxy, TLS, SASL and IMAP options of the configuration unread

#### Scenario: A block for a compiled-out backend
- GIVEN a Maildir-only build and an account whose `imap.sort` block carries an unknown key
- WHEN `account list` reads the configuration
- THEN it is refused naming that key, as a full build refuses it

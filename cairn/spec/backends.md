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
IMAP, Maildir and mbox SHALL answer `hasAttachment` by the rule of the pimdir mark (STORAGE Annex A.1) over the leaf parts of a message. A part SHALL mark the message when its `Content-Disposition` is any but `inline`. A part stating none SHALL mark it unless it is text, multipart or named by a `Content-ID`. An RFC 1847 protocol part SHALL never mark it: any child of a `multipart/encrypted`, any child but the first of a `multipart/signed`. A `Content-Type` that is no pair of RFC 2045 tokens SHALL read as `text/plain`. A `message/rfc822` part is a leaf, the parts of the attached message being its own. The `attachment` search clause SHALL read the same mark on the backends that search locally. JMAP and Graph SHALL relay what the server says; Gmail SHALL leave it `null`.

`attachment list` SHALL stay wider than this mark: it also lists signatures and images named by a `Content-ID`, and some text parts.

#### Scenario: A PDF without a disposition
- WHEN a message holds a single `application/pdf` part with no `Content-Disposition`
- THEN `envelope list --has-attachment` reports `hasAttachment: true` on IMAP, Maildir and mbox

#### Scenario: An inline PDF
- WHEN a `multipart/mixed` message holds a text part and an `application/pdf` part with `Content-Disposition: inline`
- THEN `envelope list --has-attachment` reports `hasAttachment: false` on IMAP, Maildir and mbox

#### Scenario: A signed message
- WHEN a `multipart/signed` message holds a text part and its signature, marked `attachment` or not
- THEN `envelope list --has-attachment` reports `hasAttachment: false` on IMAP, Maildir and mbox

### Requirement: The envelope carries its threading pointers
The shared `Envelope` SHALL carry `message_id` and `in_reply_to`, the RFC 5322 §3.6.4 identity of a message and of the message(s) it replies to, so a client can pair a reply with its parent from a listing rather than by reading bodies.

`in_reply_to` SHALL be a list, the grammar being `1*msg-id`, and every id in it SHALL be normalised exactly as `message_id` is (angle brackets and surrounding whitespace stripped), so the two compare byte-for-byte whatever backend surfaced them.

Each backend SHALL source the field from the response its listing already makes, and SHALL leave it empty rather than issue a request of its own: IMAP from the `ENVELOPE` (RFC 3501 §7.4.2, 9th element), JMAP from the `inReplyTo` property of `Email/get`, Gmail from the metadata headers, Maildir and mbox from the parsed message, and pimdir from the stored summary. Graph leaves it empty, `In-Reply-To` living in `internetMessageHeaders`, which a listing selection does not return.

The field SHALL NOT be printed by the `envelope list` text output, where raw msg-ids would be noise; it rides the JSON output.

### Requirement: An IMAP envelope reads its strings as a header
The subject and display names of an IMAP `ENVELOPE`, as `envelope list`, `envelope search`, `imap fetch` and `imap thread` print them, SHALL be read as `message read` reads a header (commands: "Malformed mail reads as its sender meant"): raw 8-bit bytes as UTF-8 when they are valid UTF-8 and as Windows-1252 otherwise, line breaks dropped, adjacent RFC 2047 encoded words of one charset joined as bytes before their charset is decoded, and UTF-7 read as written. A group marker of an `ENVELOPE` address list (RFC 3501 §7.4.2: the start of a group, its host `NIL` and its mailbox the group's name, and its end, all `NIL`) SHALL be no address, in `envelope list`, `envelope search` and `imap fetch` alike: an empty group lists none, as `To: undisclosed-recipients:;` does on Maildir and mbox, and a group lists its members.

#### Scenario: An emoji split across two encoded words
- GIVEN an IMAP message whose `ENVELOPE` subject is `=?UTF-8?Q?=F0=9F?= =?UTF-8?Q?=98=82?=`
- WHEN `envelope list` prints it
- THEN its subject is 😂, as `message read` shows it

### Requirement: Network backends
IMAP, JMAP, Gmail and Microsoft Graph SHALL each adapt their io-* high-level client. IMAP reuses io-imap's `select`/`fetch`/`store`/`copy`/`move`/`append`/`list`/`status`. JMAP reuses io-jmap's `mailbox_get`/`email_query`/`email_get`/`email_set`/`email_import`/`email_submission_set`/`blob_upload`/`blob_download`, addressing mailboxes by their JMAP id. Gmail treats labels as mailboxes over io-gmail's `labels`/`messages` surface; Graph treats mail folders as mailboxes over io-msgraph's `mail_folders`/`messages` surface.

### Requirement: IMAP move without the MOVE extension
The IMAP adapter's `move_messages` SHALL use `UID MOVE` (RFC 6851) when the server advertises `MOVE`. Otherwise it SHALL `UID COPY` the set to the target, then flag `\Deleted` the source UIDs really copied and, when the server advertises `UIDPLUS` (RFC 4315), `UID EXPUNGE` them. Without `UIDPLUS` it SHALL skip the expunge, leave the messages flagged in the source, and log that at debug. Nothing is flagged or expunged when the copy affected nothing. A plain `EXPUNGE` SHALL NOT be issued, so unrelated `\Deleted` messages stay.

### Requirement: An IMAP trash delete says how the trash deletes
The IMAP adapter's `delete_messages` SHALL flag `\Deleted` the given UIDs and, when the server advertises `UIDPLUS`, `UID EXPUNGE` those it found, sending no expunge when it found none. `message delete` in the trash SHALL report `deleted` when the server advertises `UIDPLUS` and `flagged` otherwise, whatever it found, so a delete that found nothing never reads as flagged. Its text SHALL advise an expunge only when it flagged messages: an expunge removes every `\Deleted` message of the trash, those other clients flagged included.

#### Scenario: An unknown id in a UIDPLUS trash
- GIVEN a trash on a server advertising `UIDPLUS`
- WHEN `message delete -m Trash 999999` names no message it holds
- THEN it prints `deleted` with `count` 0 and a `message-not-found` entry, sends no `UID EXPUNGE`, advises no expunge, and exits 3

### Requirement: IMAP copy and move counts
`copy_messages` and `move_messages` SHALL count the source UIDs a `COPYUID` reports, a `COPYUID` whose UIDVALIDITY is `0` (forbidden by RFC 4315, sent by Mailo) included. Without a readable `COPYUID` (none sent, as by a server without `UIDPLUS` or a `UIDPLUS` server when nothing matched, or one malformed beyond that UIDVALIDITY) they SHALL count the requested UIDs a `UID SEARCH UID` finds in the source: after the `UID COPY`, which leaves the source untouched, or before a `UID MOVE`, which then moves only those and is not sent when none is present. A tagged `OK` alone SHALL NOT count as a message copied or moved. A `COPYUID` counting more messages than UIDs were asked SHALL be read as none sent, its two sets being ranges a server is free to announce in the billions.

### Requirement: IMAP flag counts
The IMAP adapter's `store_flags` SHALL return how many of the given UIDs exist in the mailbox. A `UID STORE` echoing as many messages as UIDs given counts them all; one echoing fewer, which a server may do for a message whose flags did not change, SHALL be followed by a `UID SEARCH UID <set>` whose matches are the count. A removal from the trash, flagging `\Deleted`, SHALL count the same way, and SHALL `UID EXPUNGE` only when it found one.

### Requirement: An IMAP answer missing lines says so
An untagged line io-imap could not decode SHALL never be dropped silently, in the unified commands and the `imap` ones alike. In a unified command printing `errors`, a FETCH line io-imap recovered in part SHALL list its message with the items that decoded and add a `message-unreadable` entry with its UID naming the items that did not, `hasAttachment` `null` when `BODYSTRUCTURE` is among them, and its message left out when `UID` is, its entry naming no id; a line io-imap skipped whole SHALL add an entry with a `null` id, `message-unreadable` for a FETCH line and `mailbox-unreadable` for a LIST line, and a STATUS line that did not decode SHALL be `mailbox-unreadable` naming its mailbox, its counts `null`. The `imap` commands SHALL take such lines with a warning counting them and naming the command, the lines skipped apart from the FETCH lines decoded in part. A `message read` whose `FETCH` answer held such a line and no body SHALL fail coded `message-unreadable`, saying the answer does not decode, never as `message-not-found`. The text of a server `ALERT` SHALL be logged as a warning.

#### Scenario: A listing with a line that does not decode
- GIVEN a mailbox one of whose messages nests seven `multipart` levels, its `BODYSTRUCTURE` refused by imap-codec
- WHEN `himalaya --json envelope list --has-attachment` reads it
- THEN the other envelopes print, that message prints with `hasAttachment: null`, an `errors` entry coded `message-unreadable` names its UID and `BODYSTRUCTURE`, and the exit status is 3

### Requirement: Network transport resilience
The network backends SHALL run over a transport that retries a stream reporting it is not ready (`EAGAIN` on Unix, `EINTR`, and the Windows spelling of an expired deadline) rather than ending the exchange on it. Each read and each write carries its own budget of one minute, so a slow but progressing transfer never runs out of it, and exhausting the budget SHALL fail with a message naming it rather than a raw errno.

Opening a connection SHALL arm a socket read deadline matching that budget, so a server going silent on an otherwise healthy connection ends the command instead of blocking forever.

### Requirement: Local storage backends
Maildir and pimdir SHALL adapt io-maildir and io-pimdir. Maildir stores added messages under `cur/` and SHALL read an entry's flags through io-maildir rather than parsing the filename itself, so the meaning of a Maildir name is decided in the library that owns the format. A Maildir store listing skips plain files beside its folders; a missing root fails, naming it. `maildir message add` SHALL refuse flags with `--subdir new`, a Maildir keeping none there, and the `--subdir` of `maildir message add|copy|move` SHALL take `cur` or `new` only, no reader listing a message in `tmp`. `maildir message add` SHALL deliver a message, `save` staying a hidden alias of it. pimdir is an offline cache the sync engine populates, io-pimdir implementing both the store and the engine: reads project the store's items with their typed mail summaries (pimdir STORAGE Annex A) without body reads, and writes are actions queued for the store's owner to apply and a later sync to propagate, never direct SQL.

### Requirement: mbox backend
The mbox backend SHALL adapt io-mbox over the full unified operation set. A mailbox is one mbox file: `mbox.root` is walked recursively, the mailbox `a/b` being the file `a/b` below it (`a.sbd/b` with `mbox.thunderbird`), and `mbox.inbox`, when set, is the spool shown as `INBOX` and marked with the inbox role. Without `mbox.inbox`, `INBOX` (any case) SHALL fail naming the key, coded `mailbox-not-found`, unless the root holds a file of that name, and a spool that is a directory SHALL be left out of `mailbox list` and fail `account check` and every use of `INBOX`, naming the key. A message command (listing, search, read, add, flag, copy, move, delete and their raw `mbox` twins) SHALL fail on a missing mbox with ``Mbox mailbox `X` not found``, coded `mailbox-not-found`, and SHALL refuse a directory or a file that does not open on a `From_` line before taking any lock, naming the file. An absolute path passed as a mailbox SHALL open that file, so any mbox reads without configuring it, except under `himalaya mcp`, which SHALL refuse it, read or write, a tool reaching no file outside `mbox.root`. `mailbox delete` and `mailbox rename` SHALL refuse the spool, however named, from the unified commands, the raw `mbox` commands and MCP alike, as IMAP refuses to delete or rename `INBOX` (RFC 3501 section 6.3.4), a rename onto the spool included, and SHALL refuse a directory, which holds mailboxes but is none. A non-empty file below the root that does not open on a `From_` line SHALL not be listed, and an append to it SHALL be refused. A mailbox name with a level starting with `.` or ending in `.lock`, `.lck` or `.msf` (and `.sbd` with `mbox.thunderbird`, refused as the folder suffix of that layout), which no listing shows, SHALL be refused by create, rename and delete. A name that is empty, holds a NUL or a `.` or `..` level, or a level ending in `.lock` or `.msf`, SHALL be refused by every command taking a mailbox, the reads included, io-mbox resolving it to no file. In the plain layout an mbox file holds no mailbox, so a mailbox created under one SHALL be refused naming `mbox.thunderbird`. In the Thunderbird layout creating a mailbox SHALL create its missing parent mboxes empty, a rename SHALL move the mbox's .sbd directory with it, refusing when the target's exists, and a delete SHALL refuse an mbox holding others, naming them. A `mbox.root` that is no directory SHALL fail `mailbox list` naming it. The raw `mbox` command SHALL expose create, rename, delete and list of mbox files, message add (`save` a hidden alias of it), copy and move, a copy or move printing the ids the messages get in the target, each message once, and the six flag letters of the `Status` and `X-Status` headers.

### Requirement: An mbox message is its content id
The mbox backend SHALL address a message by its io-mbox content id, a hash of the message with the flag and metadata fields left out, so a flag write by Himalaya or any other client keeps it. An id the index no longer holds SHALL fail naming the mailbox, coded `message-not-found`. An append SHALL report the id the synced index gives the new message, numbered like any duplicate. A read SHALL leave out the fields the store owns: `Status`, `X-Status` and `X-Keywords`, which the flags render, Thunderbird's `X-Mozilla-Status`, `X-Mozilla-Status2` and `X-Mozilla-Keys`, and the `Content-Length` of mboxcl and mboxcl2, so `message read --raw` gives back the message added.

### Requirement: mbox flags interoperate
Shared flags SHALL map onto `Status: R` (seen), `X-Status: A`, `F`, `T` and `D` (answered, flagged, draft, deleted), any other flag going to `X-Keywords`. A keyword `X-Keywords` would not give back, one that is empty, longer than 255 bytes or holding a whitespace, a comma or a control character, SHALL be refused before any write. A message marked seen SHALL be marked old (`O`) too, as mutt writes it, since GNU mail lists a `Status: R` alone as new. A unified `flag set` SHALL keep the `O` a message carries, no shared flag naming it. A flag write SHALL keep the read, replied, flagged and deleted bits of an existing `X-Mozilla-Status` in step, and an existing `X-Mozilla-Keys` holding the keywords `X-Keywords` holds, and a mailbox delete or rename SHALL remove the mbox's Thunderbird `.msf` summary.

### Requirement: mbox writes lock and rewrite in place
Every mbox write SHALL go through io-mbox under the dotlock and the fcntl lock, each one skippable through `mbox.lock.dotlock` and `mbox.lock.fcntl`, waiting `mbox.lock.timeout` seconds for a busy one. Flag changes and removals SHALL rewrite the file in place, the whole set of messages a command names in one rewrite. A move SHALL copy then remove, so an interruption leaves the messages in both files. A lock another process holds past the timeout SHALL fail with `store-busy`, and a dotlock the user may not create SHALL fail naming `mbox.lock.dotlock = false`. A `Content-Length` (mboxcl, mboxcl2) SHALL be trusted only when its range ends on a message boundary, a blank line then a `From_` line or the end of the file.

### Requirement: A local listing holds a few messages at a time
A Maildir listing or search SHALL read its message files 64 at a time, building each one's envelope, and matching it against the filter, before the next files are read, so it holds the envelopes of the mailbox and the bytes of 64 messages at most, never every file. An mbox listing or search SHALL hand each message's bytes to the filter and drop them before the next message is read, a listing with no body clause reading no bytes the cache answers for.

### Requirement: mbox reads go through a cache
The mbox backend SHALL keep, per file, the io-mbox scan index and the envelope of every message already parsed, or that it holds no header field, under `<XDG cache>/himalaya/mbox/`, a directory created and kept at mode 0700, each file written at 0600 whatever the umask, the cache holding the subject, sender, recipients and Message-ID of every message. Every operation SHALL sync the index first, so an unchanged file is not scanned and a delivery is scanned alone, and a listing SHALL parse only the messages the cache lacks. Saving it SHALL take one pass over the index and one over the envelopes, whatever their number. The cache SHALL be disposable: a missing, stale or unreadable one is rebuilt, failing to write one is not an error, and a read, a listing or a search finding an entry stale SHALL rebuild the index and retry once. A flag change or a removal the file refuses as stale, before anything is written, SHALL rebuild the index, plan its edits again from it and write once more, the file then being scanned whole under the lock.

### Requirement: A Maildir is named relative to the root
A Maildir mailbox SHALL be named, in `mailbox list` and `maildir list`, by its path relative to `maildir.root` (`[Gmail]/Archive`), the name `-m` takes, so nested Maildirs sharing a last segment stay apart. The root Maildir SHALL list as `INBOX`, marked with the inbox role, or as `.` when a top-level folder named `inbox` in any case is a Maildir, that folder then carrying the inbox role; `-m` SHALL take that name, `.`, or the root's absolute path. A mailbox SHALL be accepted by its name, by its absolute path (the id `mailbox list` prints), with `.` segments or a trailing slash, every spelling resolving to one id, so a command comparing two mailboxes (the trash `message delete` is given, a move's source and target) sees them equal; a `..` component and an absolute path outside the root SHALL be refused. An omitted mailbox, `INBOX` or `inbox` naming no folder SHALL resolve to a top-level folder named `inbox` in any case, else the root when it is a Maildir. `maildir.layout` SHALL select the fs layout (the default, a directory per Maildir nested as the hierarchy is) or `maildir++` (Dovecot, Courier), where the root is the inbox, `.a.b` lists as `a/b` and is taken under either spelling, a level holding a dot is refused, and a rename or delete of `a` takes every `.a.*` folder along, as the fs layout moves the subtree, a rename landing one on an existing folder being refused. A missing Maildir SHALL read ``Maildir mailbox `X` not found``, coded `mailbox-not-found`. With `maildir.keywords.dovecot`, a copy or move SHALL rename the keyword letters through the target's `dovecot-keywords` file, so a keyword keeps its meaning. A unified move whose source and target are one Maildir SHALL change nothing, the ids kept. A listing or search SHALL skip, with a warning, a message file it cannot read, a folder it cannot read, and a symlink back to a folder already reached; a folder named with `-m` that cannot be searched SHALL fail with the permission error naming the path, not as a missing `cur`. Only a `2,` info section SHALL be read as flags, a flag write refusing an entry carrying any other.

### Requirement: Maildir surfaces custom keywords on demand
The Maildir backend SHALL surface custom (non-IANA) keywords on read when told which convention the mailbox uses, so a keyword written by dovecot, mbsync, OfflineIMAP, mutt or notmuch matches a `flag <name>` search as it does on the network backends. `maildir.keywords.dovecot` SHALL resolve the lowercase info-section slot letters through the resolved mailbox's own `dovecot-keywords` file, and `maildir.keywords.header` SHALL read keywords from `X-Keywords` (comma-separated) or `X-Label` (space-separated). Both default to off, and with both off the flag set SHALL be exactly the six standard info-section letters as before.

Keyword reading is not a round trip: no command can name a custom keyword, so a `FlagOp::Set` store SHALL replace the whole set and drop any keyword the message carried.

A sidecar that is absent, unreadable or disabled SHALL yield no keywords rather than fail the listing, since a mailbox without one is the normal case rather than an error.

On a Maildir account, `flags.important` and `flags.junk` SHALL be `none` unless `maildir.keywords.dovecot` is on, the header mode only reading, so the `important` and `junk` flags are refused before any write.

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

A value matching no collection id of the account SHALL be taken as a role name when it is one (`inbox`, `sent`, `drafts`, `trash`, ...), addressing the account's mailbox whose recorded role (`collections.role`) it is, the store recording a role on one mailbox of an account, the last its sync engine recorded it on. A collection's name SHALL NOT address it: two sources may hold mailboxes of one name. A value matching neither SHALL be refused naming the mailboxes the account holds. It SHALL NOT be passed to the store unresolved, which reads as a mailbox that exists and is empty.

#### Scenario: The inbox by its role
- GIVEN a pimdir account whose store records the role `inbox` on `imap/INBOX`
- WHEN `envelope list` runs without `-m`
- THEN the envelopes of `imap/INBOX` are listed

#### Scenario: The trash by its role
- GIVEN a pimdir account with no `mailbox.alias.trash` and a mailbox recorded with the role `trash`
- WHEN `message delete` runs on a message of another mailbox
- THEN the message is staged as a move to that mailbox

### Requirement: pimdir reads one account
The pimdir backend SHALL show the collections of one account (pimdir STORAGE §9.2), `pimdir.account` naming it. Unset, it is derived from the accounts holding mail: a store where one account, or one ungrouped set, holds mail is read as that one, whatever contacts or calendars other accounts hold, and a store where several hold mail is refused coded `store-account-ambiguous`, naming them, rather than guessing one and showing the wrong mailbox set. A `pimdir.account` a store holding collections holds none of SHALL be refused coded `store-account-unknown`, rather than read as an empty account, `account check` included; on a store holding no collection yet, a first sync not having run, it SHALL be read as named, its mailbox set empty. The accounts SHALL be read through io-pimdir's `PimdirReader::accounts_of`, a collection's kind through `collection::media_type`, an undeclared kind belonging to no account of mail. A root holding no store SHALL be refused coded `store-uncreated`, as the other pimdir programs refuse it. A mailbox the account does not hold SHALL be refused coded `mailbox-not-found`, and a message id that is no number coded `message-not-found`.

#### Scenario: A configured account before the first sync
- GIVEN a pimdir store holding no collection and `pimdir.account = "work"`
- WHEN `himalaya --json mailbox list` runs
- THEN it prints `{"mailboxes":[],"errors":[]}` and exits 0, where a store holding collections of other accounts alone refuses `work` coded `store-account-unknown`

### Requirement: pimdir is a reader and a producer, never the owner
The pimdir backend SHALL treat the store as a possibly-partial cache owned by the sync engine. `get_message` on an item whose body is not local (`level < Full`, no stored object, or a stored object whose blob file is gone, pimdir GUIDE §14) SHALL fail with the code `body-pending` (commands, "A JSON error carries a stable code"), the cue to sync, not a data-loss error; the item still lists.

Reads SHALL go through `PimdirReader`, the role that takes no lock (pimdir STORAGE §8) and carries no write at all, so a sync in flight neither blocks Himalaya nor is blocked by it, and the backend cannot drain the queue or sweep the store even by mistake.

The store's search index (index.db) SHALL be opened read-only, never created, rebuilt nor refreshed: the sync engine owning the store builds and refreshes it (search, "pimdir searches through the store's index").

An envelope SHALL be built from the item's typed mail summary and the addresses joined to it (pimdir STORAGE Annex A.1, A.6): every `From` and `To` address with its display name, the `In-Reply-To` list, the date, the size and the attachment mark as stored, with no body read. `hasAttachment` SHALL be `true` or `false` as the store marks it, and `null` on a row that holds no mark. A listing SHALL take the store's own order, newest first, the mail sort key being the date.

The reader SHALL overlay the queue (pimdir STORAGE §15.4), so an action this client staged is visible on the next read rather than on the next sync: a staged `set-flags`, `update`, `remove`, `move` or `copy` changes what a listing shows. A row the owner would park folds into nothing: a `remove` or a `move` staged on a message a sync left in conflict leaves it listed where it was, and a `move` brings it into no other mailbox. Each addresses a message that already exists and keeps its public id, so a staged write never changes how a message is addressed.

A write SHALL be staged as a queued `PimdirAction` through a producer handle (`store_flags`→`SetFlags`, `add_message`→`Add`, `copy_messages`→`Copy`, `move_messages`→`Move`, `delete_messages`→`Remove`), addressed by the public `seq`, for the store's owner to apply and push. The backend SHALL NOT write the index, load a collection, or run the owner's object sweep: a sweep run beside a sync destroys the bodies it has streamed but not yet attached, which STORAGE §14 explicitly invites it to leave pending. A body an action references SHALL be written to the blob store in the transaction appending the action, the queue row being what pins it. The rows one flag change, copy, move or delete stages SHALL be appended as one batch (pimdir STORAGE §15.1, io-pimdir `enqueue_batch`), each passing the capability gate (§15.6) once: an id the gate refuses, or the store failing, leaves none of them staged, the failure naming the id the gate refuses, so a rerun stages each row once.

`SetFlags` carries the whole set the message is left with. An add or a remove, the `\Seen` a read stages included, SHALL also state as `base` the set it was read from (pimdir STORAGE §15.3), so the owner applies the change alone to the set the message holds by then and a flag a sync landed or removed while the row waited survives; a `set` SHALL state none, replacing whatever the message holds. A `Remove` SHALL name no body it read (`if_match`) and resolve no conflict (`resolves`): it deletes whatever the message holds, and the owner parks it over a message a sync left in conflict, Himalaya deciding no conflict. A `Move` states no resolution either (pimdir SYNC §7), so the owner parks it over such a message too, the move `message delete` makes to the trash included, nothing staged in either mailbox: its row reads `parked` in `pimdir queue show` and in the `pimdir queue list` of the mailbox the message stays in, its error naming the diverging body. A `Copy` meets no conflict. An add or a remove on a set the store reports as unknown SHALL be refused, saying the message's flags were never synced: it has no markers to build on, and the replacement would erase whatever the server holds. A `set` replaces the whole set, so it SHALL stage one regardless. A change whose replacement set equals the set the message shows SHALL stage nothing for it, the message still counting as found. pimdir has no native trash.

An added message SHALL derive its link id through io-pimdir's mail derivation (`summary::mail::derive`), the one implementation of STORAGE Annex A.1, which is the bare `Message-ID` with nothing prepended, and SHALL name it on the queued `Add`. The action carries no summary: the owner derives the summary and the sort key from the body when it applies the action, through the same call, so the two never disagree. An `Add` whose link id the mailbox already holds live, or has an `add` queued under, SHALL be refused with the code `item-duplicate` through io-pimdir's `enqueue_with_body_checked`, and nothing SHALL be queued nor written: the owner would park such a row (pimdir STORAGE §15.3) rather than deduplicate against the stored copy or mint a second key. Minting is the store's answer to what a source hands over; refusing is its answer to a producer authoring a message the collection already has. An `add` still parks when the source lists the same `Message-ID` after it was staged.

#### Scenario: A move over a message in conflict
- GIVEN a pimdir draft a sync left in conflict
- WHEN `himalaya message move` queues it to another mailbox, and the sync engine then drains the queue
- THEN the draft stays listed in its mailbox and the other mailbox never lists it, before the drain and after
- AND `pimdir queue list` on its mailbox and `pimdir queue show` on its row read `parked`, the error naming the diverging body

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
A pimdir write carrying a body (`add_message`, `send_message`) SHALL hand the body to its producer with the action in one step (`enqueue_with_body_checked`): the capability gate and, for an add, the duplicate check run on the bytes in hand, then the body is written inside the producer's staging window (pimdir STORAGE §8) and the row pinning it appended, in one transaction. A write refused by the gate or the duplicate check SHALL leave no body in the blob tree. No body SHALL reach the blob tree while a collector holds the lock: the write waits for it. The gate reads a collection's own row before its source's (pimdir STORAGE §15.6): a source declaring `mail.message.add` `none` and `full` on the folder of its drafts role, as neverest declares a Graph source, SHALL see an add to any other mailbox refused with `unsupported`, naming the mailbox and the declared detail, nothing queued, rather than queued for every sync to reject.

#### Scenario: A collector running beside a staged add
- GIVEN a pimdir store whose collector holds `objects.lock` exclusively
- WHEN a message is added to a mailbox
- THEN no body is written until the collector releases the lock, and then the body lands and its `add` is queued

#### Scenario: A refused add
- GIVEN a pimdir mailbox holding a message whose `Message-ID` is `d1@x.org`
- WHEN another body with that `Message-ID` is added to it
- THEN the add fails with `item-duplicate` and its body is not in the blob tree

#### Scenario: An add where the source adds nothing
- GIVEN a pimdir account whose `msgraph` source declares `mail.message.add` `none` with the detail `Drafts only`, and `full` on `msgraph/Drafts`
- WHEN `himalaya message add -m msgraph/Archive` stages a message
- THEN the command fails with the code `unsupported`, naming `msgraph/Archive` and `Drafts only`, and neither a row nor a body is left; the same add to `msgraph/Drafts` is queued

### Requirement: A pimdir store failure carries its code
A failure io-pimdir reports SHALL keep io-pimdir's message as a cause and carry, under `--json`, the code `PimdirError::code()` gives it, spelled the same, when it is one a caller acts on: `store-version`, `store-uncreated`, `store-foreign`, `store-corrupt`, `store-not-current`, `store-hash-algo`, `sync-version`, `store-busy`, `store-owned`, `store-staging`, `store-in-flight`, `store-exposed`, `store-reset`, `lock-replaced`, `item-duplicate`, `flag-invalid`, `action-malformed`, `conflict-unread`, `unsupported`, `no-performer` and `ambiguous-performer`. An SQL, I/O or JSON failure carries none. Himalaya SHALL NOT derive a code from a message, and SHALL NOT advise deleting the store.

A store of another draft of the format (pimdir STORAGE §4.2), earlier or later, or one naming no format version, is `store-version`: drafts are exact, so it opens under no other draft and nothing reconciles or migrates it. Its message SHALL be io-pimdir's one wording, `version::PimdirFormatRefusal` naming `himalaya`, which orders the two versions by pimdir STORAGE §4.2 and says which is the earlier: a store an earlier sync engine wrote is synced with that engine until nothing is queued, then its directory moved aside and a new store synced; a later one asks to upgrade himalaya; one whose version the order does not read says both. It never says to delete the store. A store whose directory or pimdir.db another user can read or write, or a link where a lock file belongs, is `store-exposed`, refused before anything in it is read, its message saying to restrict it.

#### Scenario: A store another draft wrote
- GIVEN a pimdir store whose `store_meta.format_version` names another draft than Himalaya's io-pimdir follows
- WHEN any pimdir command opens it
- THEN the command fails with the code `store-version`, naming both drafts, saying the store is the earlier and to sync it out then move its directory aside, or the later and to upgrade himalaya, and never to delete it

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
Himalaya SHALL carry a `pimdir` subcommand for what the operator CLI cannot do without knowing mail. `queue list` SHALL list every pending row of the account's queue, whatever its kind (`add`, `submit`, `set-flags`, `move`, `copy`, `remove`, `update`, the collection intents, an application's own), and its parked rows of any kind, mailbox by mailbox in store order, or those of the one mailbox `-m/--mailbox` names (a name, alias, role or id), where the kind-agnostic `pimdir` binary can only print ids and hashes.

Under `--json` it SHALL print `{"actions": [<row>], "parked": [<row>], "errors": [<item error>], "refused": [<refusal>]}`, the pending rows then the parked ones, each in append order and in the family's one queue row (pimalaya-cli cairn/spec/output.md, *A queue row keeps one shape*), every key printed: `{"row", "handle", "kind", "collection", "linkId", "itemId", "producer", "queuedAt", "attempts", "unconfirmed", "error", "subject"}`. `row` is the queue row id, an integer, and `handle` the row under the store's identity (`<store id>.<incarnation>.<row>`, read from `PimdirReader::change_cursor`, pimdir STORAGE §4.5), either of which `queue show` and `queue cancel` take, the text printing the handle in place of the bare id; `kind` the row's action as the store holds it, read from io-pimdir (`PimdirAction::kind` for a pending row, `PimdirParkedAction::action` for a parked one), never re-spelled; `collection` the mailbox's id as `mailbox list` prints it; `linkId` the link id of the message an `add` creates, the `id` its receipt printed, `null` for any other row, a `submit` among them, the store creating no message for it; `itemId` the public id of the stored message a `set-flags`, `move`, `copy`, `remove` or `update` acts on, as `envelope list` prints it, `null` for any other row; `producer` the process that staged the row; `queuedAt` the instant the store stamped it; `attempts` how many times the sync engine tried it, an integer from 0 (`uint64`, minimum 0, in its JSON Schema); `unconfirmed` `true` for a row whose outcome is unknown (pimdir STORAGE §15.2); `error` `null` while the row is pending and the store's text once it is parked; `subject` the subject of the body the row pins, else of the message it acts on, a parked `submit` naming no body giving its payload's subject, `null` when there is none to read. The queued action carries no summary, so a subject SHALL be derived from the body the action pins, through the same derivation the owner applies. A parked payload that does not decode addresses nothing: `linkId`, `itemId` and `subject` are `null`. A pending row whose payload does not decode leaves its mailbox's pending rows unread, io-pimdir reading a collection's pending rows whole: the mailbox SHALL be one item error of `errors`, its id the mailbox's, coded `action-malformed`, the other mailboxes still listed, and the command SHALL exit 3. No placeholder SHALL stand in for the id a queued message lacks (no `""`, `0` nor row id).

`refused` SHALL list every refusal standing in what is listed, the account or the one mailbox, (io-pimdir's `PimdirReader::refusals`, pimdir STORAGE §15.4), per message and source, `{"collection", "itemId", "source", "reason", "refusedAt"}`: a change the store applied that a source's server refused for good, which the sync engine holds until the message is changed again or removed. The text form SHALL list each after the parked rows, `refused` in the state column, its instant, the message id, then the source and the reason; `Cancel:` SHALL print only when a pending or parked row is listed, a refusal being no row to cancel. The header SHALL name the mailbox only when `--mailbox` narrows the listing.

#### Scenario: A queued send
- GIVEN a pimdir account holding `imap/INBOX` and `imap/Sent`, and a send queued on `imap/Sent`
- WHEN `himalaya --json pimdir queue list` runs, then `himalaya --json pimdir queue list -m imap/INBOX`
- THEN the first lists the `submit` row of `imap/Sent` under `actions`, and the second lists no row

#### Scenario: Every kind is listed
- GIVEN a pimdir account where `message add`, `message send`, `flag add` and `message move` ran
- WHEN `himalaya --json pimdir queue list` runs
- THEN `actions` holds an `add`, a `submit`, a `set-flags` and a `move` row, the last two naming the message as `itemId`, `parked` and `errors` empty

`queue cancel` SHALL retract one pending or parked row of the account's queue, confirming first unless `--yes`, and SHALL name the row's action kind in its prompt and output: `{"row", "handle", "kind", "unconfirmed"}`, `row` the row id and `handle` the row under the store's identity, a bare id given named under this store's. An unconfirmed row (pimdir STORAGE §15.2) is cancelled as a parked one, which undoes nothing its performer sent: the prompt and the plain output SHALL say it was sent and may have taken effect, and `unconfirmed` is `true`. A row anchored on a collection that is not one of the account's mailboxes (another account's, or an address book or calendar another app of the store queued on), or one neither pending nor parked, SHALL be refused before anything is opened or asked, and stays as it was, coded `row-not-found` unless it was applied. With no terminal to answer and no `--yes`, the cancel SHALL be refused coded `confirmation-required` before anything is opened.

Taking the owner role briefly is what cancelling costs (pimdir STORAGE §15.5); the backend read and write paths SHALL NOT reach it. The cancel SHALL open the store only as it stands (io-pimdir `open_current`): it SHALL NOT create, migrate nor reconcile the schema. A store of another draft SHALL be refused as `store-version` and a damaged one as `store-corrupt`, each left as it is. A store another process owns SHALL be refused immediately, saying a sync is running and that the action may already have been applied.

#### Scenario: A queued creation is listed from its body
- GIVEN a queued `add` of the message `d1@x.org` pinning a body whose `Subject:` is `hi`
- WHEN the queue is listed with `--json`
- THEN its row prints `"kind":"add"`, `"linkId":"d1@x.org"`, `"itemId":null`, `"error":null` and `"subject":"hi"`, and no `send` nor `envelope`

#### Scenario: A parked move names its message
- GIVEN a `move` of message 7, whose subject is `hi`, the sync engine parked
- WHEN the queue of its mailbox is listed with `--json`
- THEN the row is under `parked` with `"kind":"move"`, `"itemId":"7"`, `"linkId":null`, `"subject":"hi"` and the store's error text

#### Scenario: A cancel on a store of another draft
- GIVEN a store whose `store_meta.format_version` names another draft
- WHEN `himalaya pimdir queue cancel` runs on one of its rows
- THEN the command fails with the code `store-version`, saying the store is the earlier and to sync it out then move its directory aside, and neither the store's stamp nor the row changes

#### Scenario: Another account's row
- GIVEN a store shared by two accounts and a row queued on the other account's collection
- WHEN `himalaya pimdir queue cancel` runs on that row
- THEN the command fails saying the row is not in the account's queue, and the row stays queued

#### Scenario: A flag change a server refused
- GIVEN a pimdir flag change the sync engine applied, then the IMAP source's server refused for good
- WHEN `himalaya --json pimdir queue show <ROW>` and `pimdir queue list` run on its row and its mailbox
- THEN the row reads `applied` on its message with one entry of `refusals` naming `imap` and the server's reason, and the mailbox lists the message under `refused`

### Requirement: pimdir writes name their queue row
`pimdir message add` and `pimdir message send` SHALL stage a message as the unified `message add` and `message send` do, and print the message's link id as `id`, the id the unified `message add` prints and the row's `linkId`, beside the queue row it waits in: `{"id": string, "row": integer, "handle": string}` for an add, plus `"copy": bool` (the `submit` row carries the copy) and `"copyRow": integer` with `"copyHandle": string` (a copy queued beside the send, when its performer declares no capabilities, both `null` otherwise) for a send. `pimdir flag add|set|remove` SHALL print the row of each message they found and what a source supports of the change only in part: `{"messages": [{"id": string, "row": integer | null, "handle": string | null}], "warnings": [string]}`, both `null` for a message the change left as it was, for which nothing was staged, each warning once, in the words of the source's declaration (`Source gmail supports mail.flags.keywords in part: …`), `Warning:` lines after the sentence in text. The unified commands SHALL keep printing no queue detail. `pimdir queue list`, `queue show` and `queue cancel` SHALL name a row `row` and `handle` as well.

`pimdir queue show <ROW>` SHALL say where a row of the account's queue stands, read through io-pimdir's `action_status` (pimdir STORAGE §15.4), printing the row `pimdir queue list` prints for it, every key of it, then what only a read carries (pimalaya-cli cairn/spec/output.md, *A queue read prints the list row*): `{"row", "handle", "kind", "collection", "linkId", "itemId", "producer", "queuedAt", "attempts", "unconfirmed", "error", "subject", "state": "pending" | "parked" | "unconfirmed" | "applied", "appliedAt", "refusals"}`, a key the state does not carry being `null`, `refusals` `[]`. A pending, parked or unconfirmed row carries the keys of its list row, a parked one `error` too; an applied one, left the queue, carries `collection`, `appliedAt` and `itemId`, the public id of the message it acted on (the one an add, a move or a copy left, the one a flag change or a removal named), the keys of its list row `null`, and `refusals`, read with the receipt (`PimdirActionStatus::Applied::refusals`, pimdir STORAGE §15.4): one `{"collection", "source", "reason", "refusedAt"}` per source whose server refused for good the change the row led to, the plain output printing `Refused by <source> in <collection> at <instant>: <reason>` under the applied line. A refused row reads `applied` all the same: its effect is in the store, so it is neither parked again nor cancelled, and the way out is an action on the message. An intent its performer sent without learning its outcome (pimdir STORAGE §15.2), a send or a mailbox change that may have taken effect, SHALL read `unconfirmed`, never `parked`, with `collection`, `kind`, `attempts` and, as `error`, what the performer recorded; the plain output says it was sent and maybe performed. A handle another store drew (§4.5), a copy restored since, where its id may name another row, SHALL fail coded `store-reset`, exit 1. A row neither queued nor in the store's receipts (kept seven days), or one anchored on a collection that is not one of the account's mailboxes (another account's, or another app's address book or calendar), cancelled or applied long ago, SHALL fail coded `row-not-found`, exit 1. A row the sync engine performed rather than applied, a message sent or a mailbox created, renamed or deleted, reads as `applied` without `itemId`, its receipt recorded on acknowledgement (pimdir STORAGE §15.5). A receipt follows the collection its row was queued on and outlives it (pimdir STORAGE §15.4): a renamed mailbox's row reads `applied` under the mailbox's new id, and a row whose mailbox is gone since, a performed deletion's included, still reads `applied`, with `collection` `null`, the plain output saying the mailbox was deleted since. Such a row names no collection an account could be told from, so it reads `applied` under any account of the store.

The plain output of every `pimdir` subcommand SHALL end with a newline.

#### Scenario: A staged draft is followed by its row
- GIVEN a pimdir account
- WHEN `himalaya --json pimdir message add -m imap/Drafts` stages a message
- THEN it prints `{"id":"d1@x.org","row":12,"handle":"e2646df031cd876334410c4df14ebd86.1.12"}`, and `pimdir queue show e2646df031cd876334410c4df14ebd86.1.12` prints its list row and `"state":"pending"` until the sync engine applies the row, then `"state":"applied"` with the `itemId` the message is read by

#### Scenario: An unknown row
- GIVEN a pimdir account whose queue holds no row 999
- WHEN `himalaya --json pimdir queue show 999` runs
- THEN it prints the failure object coded `row-not-found` and exits 1

### Requirement: A queue row is named by its handle

Every pimdir output naming a queue row SHALL print `row`, the row id, an integer, and `handle` beside it, the row under the store's identity, io-pimdir's `PimdirActionHandle` (pimdir STORAGE §4.5, §15.4): the store id, the incarnation and the row id joined by periods (`e2646df031cd876334410c4df14ebd86.1.42`), as Calendula and Cardamum print them (pimalaya-cli output.md, *A queue row keeps one shape*). This holds for the receipts of `pimdir message add|send` (the copy's `copyRow` with `copyHandle`), `pimdir mailbox create|rename|delete`, `pimdir flag add|set|remove` and `pimdir tag`, `null` both where nothing was staged, for the rows of `pimdir queue list` and the parked rows of `pimdir status`, and for `pimdir queue show` and `pimdir queue cancel`. `pimdir queue show <ROW>` and `pimdir queue cancel <ROW>` SHALL take such a handle, checked against the store's identity, or a bare row id, followed unchecked; their `handle` SHALL be the handle given, or the bare id given named under the store's identity. A handle another store drew, a copy restored since, reads `reset` in `queue show`, and `queue cancel` SHALL refuse it with the code `store-reset` before it asks or opens the store as owner, the row its id names left queued. Anything that is neither a handle nor an id SHALL be refused as a usage error (exit 2).

#### Scenario: A handle of a restored copy
- GIVEN `pimdir message add` printed `"handle":"e2646df031cd876334410c4df14ebd86.1.12"` and the store was since restored from a copy of another incarnation
- WHEN the user runs `pimdir queue cancel e2646df031cd876334410c4df14ebd86.1.12 --yes`
- THEN the command fails with the code `store-reset` and the row 12 of the restored store stays queued

### Requirement: A pimdir refile or delete stages the ids it found
`copy_messages`, `move_messages` and `delete_messages` SHALL check every given id against the mailbox before staging the first row, skip an id the mailbox does not hold, stage one row per id found and report how many they found (commands, "A command given message ids says how many it found"). A call finding none SHALL stage nothing. A malformed id SHALL fail the call before anything is staged.

#### Scenario: One id of two exists
- GIVEN a pimdir mailbox holding message 3 and no message 999
- WHEN `message copy --from imap/INBOX --to imap/Trash 3 999` runs
- THEN one `copy` row is queued for 3, and the command succeeds reporting one message

### Requirement: The pimdir queue view shows parked rows
`himalaya pimdir queue list` SHALL list the parked rows of what it lists, any action kind, beside the pending rows, in the one queue row ("The pimdir subcommand reads and retracts the queue"), under `parked` in JSON: the row id, the action kind, the message it acts on, when it was queued, whether it is unconfirmed (`unconfirmed`, pimdir STORAGE §15.2) and the error that parked it. The plain output SHALL list an unconfirmed row as `unconfirmed` where it lists another as `parked`. `queue cancel` SHALL take a parked row as it takes a pending one.

#### Scenario: A duplicate draft parks
- GIVEN a staged `add` the sync engine parked because the source listed the same `Message-ID` before applying it
- WHEN `himalaya pimdir queue list -m imap/Drafts` runs
- THEN the row is listed as parked with its error, and `pimdir queue cancel` on its id removes it

### Requirement: pimdir status reports the store
`himalaya pimdir status` SHALL report what the pimdir store holds of the account, as Calendula's and Cardamum's `pimdir status` report theirs, and print `{"account", "accounts", "mailboxes", "parked", "refused"}`: `account` the account the store groups this account's mail under (`pimdir.account`, configured or derived), `null` in a store grouping none; `accounts` every account the store groups collections under (`PimdirReader::list_accounts`); `mailboxes` one `{"id", "name", "total", "hydrated", "queued"}` per mailbox of the account, by id, `name` its id when it has none, `total` its live messages (`count_items`), `hydrated` those carrying a local body, read off the summaries with no body opened, `queued` its queued creations and sends, those `pimdir queue list` lists under `messages`; `parked` each row the sync engine parked in those mailboxes, in append order, `{"row", "handle", "mailboxId", "kind", "error"}`, `kind` as the store holds it; `refused` each refusal standing in them (pimdir STORAGE §15.4) `{"mailboxId", "itemId", "source", "reason", "refusedAt"}`, the row `pimdir queue list` prints under its `refused` with the mailbox named `mailboxId`, as Calendula and Cardamum name theirs by `calendarId` and `addressbookId`; `pimdir queue list` keeps its own keys. The text SHALL print `Account:` (the account and `(pimdir)`), `Store account:` and `Grouped accounts:` above one line per mailbox (its id, `<hydrated>/<total>`, `<queued> queued`, its name), then a blank line and one line per parked row (its handle, `parked` or `unconfirmed`, its kind, its mailbox, its error), then a blank line and one line per refusal (the message id, `refused`, the source, the mailbox, the reason), each block printed only when it holds a line.

#### Scenario: A parked move in the report
- GIVEN a pimdir account holding `imap/Drafts` and `imap/Trash`, and a move of a draft the sync engine parked
- WHEN `himalaya --json pimdir status` runs
- THEN it prints both mailboxes by id, `imap/Drafts` with its `hydrated` and `total`, and under `parked` the move's `row`, `handle`, `"mailboxId":"imap/Drafts"`, `"kind":"move"` and its error

### Requirement: pimdir creates a mailbox by queueing an intent
`pimdir mailbox create <NAME> [--parent <MAILBOX>]` SHALL enqueue one `collection-create` intent (pimdir STORAGE Annex B.2) with the `v: 1` payload `{"v": 1, "source": id, "name": text, "parent": collection?}`, and print `{"id": null, "row": integer, "handle": string, "name": text, "parent": collection | null, "source": id}`, `id` `null` until the sync performing the row brings the mailbox down, `parent` `null` at the top, never left out. `pimdir mailbox rename` and `pimdir mailbox delete` SHALL print `id` too, the id of the mailbox they rename or delete, beside their row. It SHALL NOT create a collection in the store: the mailbox arrives with the sync that performs the intent.

The row SHALL be anchored on `parent`, which must be a mailbox of the account, else on the account's first mailbox by id, and queued through io-pimdir's `enqueue_collection_create`. The performer SHALL be named as for a send (capability `collection.create`): several candidates and no recorded choice are refused, naming them. A store whose sources declare no capabilities SHALL be refused coded `capability-missing`, its owner predating the intent, as SHALL a renaming or a deletion on such a store. An empty name, or one holding a control character, SHALL be refused before anything is staged. A mailbox the store lists already SHALL be refused with `mailbox-duplicate`, naming it, before anything is queued, as "Unified mailbox writes" refuses creating an existing mailbox: one the performer's source syncs under NAME at the top, or under the parent's name, a hierarchy delimiter (`/` or `.`, the store recording none) and NAME, names compared exactly; so SHALL a creation of NAME under that parent the same source has queued already. A name the store does not list yet is the server's to refuse: the row parks, and `pimdir queue show` says why.

#### Scenario: A folder is created through the sync engine
- GIVEN a pimdir account whose `imap` source declares `collection.create`
- WHEN `himalaya --json pimdir mailbox create Projets` runs
- THEN one `collection-create` row is queued with `{"v":1,"source":"imap","name":"Projets"}`, and the command prints its `row` and `"source":"imap"`

#### Scenario: A mailbox at the top
- GIVEN a pimdir account whose `gmail` source declares `collection.create`
- WHEN `himalaya --json pimdir mailbox create Projets` runs
- THEN it prints `"parent":null` beside `row`, `name` and `source`

#### Scenario: A mailbox the store lists
- GIVEN a pimdir account whose `imap` source declares `collection.create` and syncs `imap/Projets`, named `Projets`
- WHEN `himalaya mailbox create Projets` or `himalaya pimdir mailbox create Projets` runs
- THEN the command fails with the code `mailbox-duplicate`, naming `imap/Projets`, and nothing is queued

### Requirement: pimdir renames or deletes a mailbox by queueing an intent
`pimdir mailbox rename <MAILBOX> <NAME>` SHALL enqueue one `collection-rename` intent (pimdir STORAGE Annex B.2) with the `v: 1` payload `{"v": 1, "source": id, "name": text}` and print `{"row": integer, "handle": string, "mailbox": collection, "name": text, "source": id}`. `pimdir mailbox delete <MAILBOX>` SHALL enqueue one `collection-delete` intent with `{"v": 1, "source": id}` and print `{"row": integer, "handle": string, "mailbox": collection, "source": id}`. Neither SHALL rename or delete a collection in the store: the change arrives with the sync that performs the intent.

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

### Requirement: pimdir performer takes the intents Himalaya queues
`pimdir performer <CAPABILITY> [SOURCE] [--clear]` SHALL take the capability of an intent Himalaya queues (pimdir STORAGE Annex B.2): `mail.submit`, `collection.create`, `collection.rename` or `collection.delete`. Any other SHALL be refused by the grammar, exit 2 and `usage-invalid` under `--json`, naming the ones it takes, rather than answering that no source performs it.

#### Scenario: An unknown capability
- GIVEN a pimdir account
- WHEN `himalaya --json pimdir performer bogus.cap` runs
- THEN it exits 2 with the failure object coded `usage-invalid`, its error listing `mail.submit`, `collection.create`, `collection.rename` and `collection.delete`

### Requirement: pimdir stages flags by their raw spelling
`himalaya pimdir flag list|add|set|remove` SHALL read and stage the flags of a pimdir message by their raw spelling, verbatim. A value naming an IANA flag SHALL be accepted only in its canonical spelling, compared without case and staged in the canonical case, a bare `seen` or `$seen` being refused naming `\Seen`; `\Deleted` and `\Recent` SHALL be refused. A backslash names a system flag, so one in front of any other word SHALL be refused, as SHALL a character IMAP forbids in a flag (`(`, `)`, `{`, `%`, `*`, `"`, `]`, `\`). A value the store cannot hold (pimdir STORAGE §13), empty or holding whitespace, a control character or a character outside ASCII, SHALL be refused with the code `flag-invalid`, the code io-pimdir's enqueue gives a flag set holding one, and nothing staged. Any other value is a keyword.

`add`, `set` and `remove` SHALL fail when no message is named or none of the named ones exists, and SHALL stage one `SetFlags` per message through the backend's `stage_flags`, appended as one batch (io-pimdir's `enqueue_batch`, pimdir STORAGE §15.1) whose enqueue runs the gate of the sources' declarations once per row (§15.6), so a change refused on any message stages nothing and the failure names that message: a source declaring `mail.flags.keywords` as `none` SHALL refuse a keyword change with its detail, and one declaring it in part SHALL queue it, its receipt carrying the declaration's warning under `warnings`, never claiming the server keeps the keyword. A change on `$Important` or `$Junk` SHALL be gated by `mail.flags.important` or `mail.flags.junk` from a source stating that row, `none` included, and by `mail.flags.keywords` from one stating none (pimdir STORAGE Annex B.1), io-pimdir's gate deciding which. `list` SHALL print the flags a message carries as the overlaying reader shows them, each the shared `{raw, iana}` flag (commands: "A local store flag prints its meaning beside its spelling"), and `null` under `--json` for a message whose flags the sync engine never read.

An add or a remove SHALL match the flags a message carries as flags, not as spellings: a marker the store holds as `\seen`, or as Thunderbird's `Junk`, is the one `\Seen` or `$Junk` keeps on an add and drops on a remove. This holds for the shared flag commands on a pimdir account, which stage through the same path.

#### Scenario: A keyword is staged and read back
- GIVEN a pimdir account whose `imap` source declares `mail.flags.keywords`
- WHEN `himalaya --json pimdir flag add -m imap/INBOX -f act 42` runs
- THEN it prints `{"messages":[{"id":"42","row":7,"handle":"e2646df031cd876334410c4df14ebd86.1.7"}],"warnings":[]}`, and `pimdir flag list -m imap/INBOX 42` lists `act` before the next sync

#### Scenario: A keyword kept in part
- GIVEN a pimdir account whose `gmail` source declares `mail.flags.keywords` in part, `$Important only`
- WHEN `himalaya --json pimdir flag add -m gmail/INBOX -f act 42` runs
- THEN the row is queued and the receipt prints `"warnings":["Source gmail supports mail.flags.keywords in part: $Important only"]`

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
`pimdir mailbox list` SHALL list the account's mailboxes sorted by id, each as `{"id", "name", "role", "total", "unread", "coverage", "round"}`: `role` as `mailbox list` shows it (`null` when none), `total` and `unread` counted as the unified `mailbox list --counts` counts them, `coverage` the collection's coverage (pimdir STORAGE §14.1, the narrowest of its sources') as `{"since", "until", "closedAt"}`, `closedAt` when the round leaving it closed, and `round` a round a source has under way as `{"since", "until", "startedAt"}`. Bounds are RFC 3339 instants, `null` for an open end. `coverage` SHALL be `null` until every source of the collection closed a round on it, and on a store its owner has not reconciled; `round` SHALL be `null` when no round is under way. The plain list SHALL show the coverage and the round as their days (`since 2026-09-07`, `all`). The unified `mailbox list` SHALL carry none of it.

#### Scenario: A mailbox synced from a date
- GIVEN a pimdir account whose `imap` source closed a round on `imap/INBOX` over the mail since 2026-09-07
- WHEN `himalaya --json pimdir mailbox list` runs
- THEN `imap/INBOX` carries `"coverage":{"since":"2026-09-07T00:00:00Z","until":null,"closedAt":…}`, and a mailbox no round closed on carries `"coverage":null`

### Requirement: pimdir reads only the page an envelope listing needs
`envelope list` on a pimdir account with a page size SHALL read the mailbox newest first by keyset pages and stop at the end of the page asked for: page N of size S reads at most N × S items, overlaid as every read is. Without a page size it SHALL read the whole mailbox.

`envelope search` SHALL read newest first and, when the order is the default one (date descending, or a sort that is `date desc` alone), stop once the page asked for and one more hit are found, that hit telling a page follows. Date clauses at the top of the query (`date`, `after`, `not after`, joined by `and`) SHALL bound the walk on the store's sort key, the mail sort key being the summary date (pimdir STORAGE §9.3, Annex A.1). `not flag seen` at the top of the query SHALL read through io-pimdir's unread chip (`list_mail_page_filtered`) when io-pimdir's reader answers that no pending action changes what a read of the mailbox shows (`PimdirReader::overlay_quiet`), the chip reading committed rows only; a queue that does not read fails the search. Every clause SHALL still be evaluated on the envelopes read, and every other sort applied to them, so a search returns the same envelopes in the same order as a whole-mailbox read. A row read whose sort key is not its summary date in the store's canonical form SHALL make the search read the whole mailbox instead.

#### Scenario: The first page of a large mailbox
- GIVEN a pimdir mailbox holding 50,000 mails
- WHEN `himalaya envelope list -s 50` runs
- THEN the store reads the 50 newest items, and one more per queued removal, rather than the 50,000

### Requirement: pimdir lists the account's attachments
`pimdir attachment list` SHALL list the files the account's messages attach, read from the stand-ins of the account's attachments collection (pimdir STORAGE §14.3) through io-pimdir's `list_attachments_by_account`, newest message first, opening only the bodies of the page's messages, each once, to number their parts. It SHALL print `{"attachments": [...], "nextPage"}`, each file as `{"id", "filename", "mime", "size", "part", "section", "message", "mailbox", "from", "date"}`, the keys `attachment list` spells alike: `id` the stand-in's public id; `filename` its decoded name, `null` when the part names none; `mime` its MIME type, lowercased without parameters; `size` in octets once the transfer encoding is undone; `part` the id `attachment download` takes and `attachment list` and `message read` print, the position of the part standing at `section` in the message's body, `null` when the body is not local or holds no leaf part there; `section` its IMAP section in the message (RFC 3501 §6.4.5), which `message parse --section` takes; `message` and `mailbox` the public id and collection of one live placement of the message attaching it, as `envelope list` and `message read` take them; `from` the message's sender as an envelope address, `{name, email}`, `null` when the store records no address; `date` its date, RFC 3339. Ids SHALL be strings.

A page SHALL hold `--page-size` files (else `envelope.list.page-size`, else 25). `nextPage` SHALL be an opaque token that `--page-token`, long only (`-t` is `--to` across Himalaya), takes to start after the page's last file, and `null` exactly when no file follows. Pages SHALL follow one another by keyset on the message's sort key and the stand-in's id, so a write between two pages neither repeats nor skips a file already listed. A token this command did not print SHALL be refused. An account with no attachments collection, or a store without stand-ins, SHALL list none rather than fail.

`pimdir attachment search <WORDS>...` SHALL print the same page, keeping the files for which every word appears, ignoring case (Unicode lowercase; accents count), in the file name, the sender's address or the sender's name. It SHALL read the listing newest first and stop once the page and one more file are found. A query of no word SHALL be refused.

#### Scenario: A page of files and the next one
- GIVEN a pimdir account whose stored messages attach four files
- WHEN `himalaya --json pimdir attachment list -s 3` runs
- THEN it prints the three files of the newest messages and a `nextPage` token, and `pimdir attachment list -s 3 --page-token <TOKEN>` prints the fourth with `"nextPage":null`

#### Scenario: A file downloads by its part
- GIVEN a pimdir message whose body is `multipart/mixed` of a `multipart/alternative` text and a PDF, the PDF at section `2`
- WHEN `himalaya --json pimdir attachment list` runs
- THEN the PDF prints `"part":"5"` and `"section":"2"`, and `attachment download --stdout -m <mailbox> <messageId> 5` writes the PDF

#### Scenario: Invoices from one sender
- GIVEN a pimdir account where `billing@shop.test` sent `Facture-Octobre.pdf` and another sender `plage.jpg`
- WHEN `himalaya --json pimdir attachment search facture billing` runs
- THEN it prints `Facture-Octobre.pdf` only

### Requirement: pimdir sends by queueing a submit intent
The pimdir backend SHALL send a message by staging its raw bytes and enqueueing one `submit` action, which whatever owns the store performs through its own channel. It SHALL NOT open a network connection to send, and SHALL NOT use the account's `smtp` section. The sources syncing the anchor SHALL count as declaring when any source of the account is a candidate performer of the capability, so a mailbox only declared (`neverest sync --declare-only`) takes the performer `pimdir performer` names.

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
`himalaya pimdir queue list` SHALL list a queued `submit` beside the queued creates, in the account's listing as in its mailbox's, its `kind` `submit`, its subject derived from the body it pins as a create's is.

### Requirement: Append gap
Graph SHALL NOT implement `add_message`: a MIME message it creates stays a draft, so it declares `message.add` `none`, and a `message add`, or a save asked of a composer or of `message send`, is refused before connecting. Gmail SHALL implement it through `messages.insert`, the mailbox and the flags becoming labels and the `Date:` header dating the message, without sending it or running filters. Every backend implements `search_envelopes` (see the search capability).

The shared `add_message` result SHALL carry an optional id, absent only when an IMAP or JMAP server acknowledges the write without reporting one. An IMAP `APPEND` acknowledged by the server SHALL stay a success when neither `APPENDUID` (RFC 4315) nor the fallback `Message-ID` search yields a UID, and a requested send SHALL go on. An `APPEND` rejected by the server SHALL stay an error.

### Requirement: Sending transport
Backends that self-send (JMAP, Gmail, Graph) SHALL route `send_message` through their own API, and pimdir through its queue. Storage backends that cannot send (IMAP, Maildir, mbox) SHALL send through the account's SMTP transport, adapted in `src/smtp/backend.rs` over io-smtp, which parses the RFC 5321 envelope from the raw message headers. An envelope address holding a control character SHALL fail the send wherever the envelope is derived from the headers, the pimdir queue included, and an address io-smtp refuses as an RFC 5321 mailbox (a space, a line break, a second `@`) SHALL fail an SMTP send before `MAIL FROM`, the error naming the address with its control characters escaped. The transmitted message SHALL NOT carry the `Bcc:` field (RFC 5322 3.6.3), which io-smtp removes after the envelope is derived. The protocol-level `smtp send` SHALL transmit its message verbatim, its envelope being explicit, and SHALL refuse a `--mail-from` or `--rcpt-to` io-smtp refuses when the command line is read, a quoted local part and an address literal being taken. Its `--notify`, `--ret` and `--envid` SHALL request delivery status notifications (RFC 3461), `NOTIFY` going on every `RCPT TO` and `RET` and `ENVID` on `MAIL FROM`; the command SHALL fail before `MAIL FROM` when the server does not announce `DSN`, and SHALL refuse `never` combined with another condition and an `ENVID` needing xtext encoding or longer than 100 characters. The unified `message send` requests none.

### Requirement: A JMAP send files the message as sent
A JMAP send SHALL stage the message in the drafts mailbox with `$draft`, then submit it with an `onSuccessUpdateEmail` patch (RFC 8621 section 7.5) moving it to the sent mailbox, unsetting `$draft` and setting `$seen`. The drafts mailbox SHALL be `jmap.drafts-mailbox-id`, else the `drafts`-role one, the send failing without either; the sent mailbox SHALL be `jmap.sent-mailbox-id`, else the `sent`-role one, the message only losing `$draft` without either. A patch the server refuses, or answers with an error, once the message is submitted SHALL be logged as a warning saying the message stays a draft, the send having succeeded. A submission the server refuses (an entry in `notCreated`) SHALL fail the send with its error and code, after destroying the staged email with `Email/set` when this send's import created it; an email the import found already there (`alreadyExists`) SHALL be left as it was. A destroy that fails SHALL be logged as a warning naming the email's id, the refusal staying the result. A submission request failing without such an answer (the connection, an HTTP status, a method error) SHALL fail the send and destroy nothing, the message possibly sent, a warning naming the id of an email this send created.

#### Scenario: A refused submission
- GIVEN a JMAP server importing the message as a new email `E1`, then refusing its submission with `invalidRecipients`
- WHEN `message send` runs
- THEN it sends `Email/set` destroying `E1`, and fails coded `recipient-refused`

### Requirement: JMAP writes are scoped to the mailbox given
The JMAP backend SHALL scope `flag add|set|remove`, `message copy`, `message move` and `message delete` to the mailbox given, reading the `mailboxIds` of the ids first (`Email/get`); an id the mailbox does not hold counts as not found. A move onto its own mailbox SHALL change nothing. A removal from the trash SHALL destroy an email only the trash holds and only take the trash out of the mailboxes of one held elsewhere too. Each id an `Email/set` refuses SHALL be an `errors` entry coded by its SetError type, the others written.

### Requirement: A JMAP read is scoped to the mailbox given
The JMAP backend SHALL read one message (`message read`, and the source of `message reply|forward` and `attachment list|download`) only when the mailbox given holds it, reading its `mailboxIds` with its `blobId` in the one `Email/get`: an email the mailbox does not hold SHALL fail `message-not-found`, naming the mailbox, as on IMAP, though a JMAP id names an email account-wide. The mailbox SHALL be resolved through the mailbox listing only when it is not among the email's mailbox ids.

#### Scenario: An inbox email read under the sent mailbox
- GIVEN a JMAP account whose email `E1` the inbox alone holds
- WHEN `himalaya message read -m sent E1` runs
- THEN the command fails with the code `message-not-found`, naming `sent`, where it printed the email

### Requirement: A JMAP duplicate import files the existing email
A JMAP server forbidding duplicates refuses an import of bytes the account holds with `alreadyExists` (RFC 8621 section 4.8): `message add` and a save SHALL then file the existing email into the mailbox with the flags and report its id, and a send SHALL submit the existing email. Any other import or submission refusal SHALL fail naming its type and description.

### Requirement: A Gmail answer joins its parent's thread
A message Gmail adds or sends answering another (`In-Reply-To:`) SHALL carry the `threadId` of the first parent the account holds, found by `rfc822msgid:`.

### Requirement: An unpaged Gmail listing reads every page
An unpaged Gmail `envelope list` or `envelope search` (`--page-size 0`) SHALL follow every next page token, asking 500 messages a page, Gmail's largest `maxResults`, and SHALL say no page follows only once Gmail served the last one.

### Requirement: A Gmail read is scoped to the mailbox given
The Gmail backend SHALL read one message (`message read`, and the source of `message reply|forward` and `attachment list|download`) only when it carries the label of the mailbox given, reading its `labelIds` in the one `messages.get` (format RAW) bringing its bytes: a message not carrying the label SHALL fail `message-not-found`, naming the mailbox, as on IMAP and JMAP, though a Gmail id names a message account-wide, and `--seen` SHALL change nothing. The mailbox SHALL be resolved through the label listing only when it is not among the message's label ids.

#### Scenario: An inbox message read under the sent mailbox
- GIVEN a Gmail account whose message `m1` carries the `INBOX` label alone
- WHEN `himalaya message read -m sent m1` runs
- THEN the command fails with the code `message-not-found`, naming `sent`, where it printed the message

### Requirement: Graph batches four requests at a time
Graph SHALL send its `$batch` requests (the well-known folder lookup, the folder check of message ids, the `PATCH`, copy, move and `DELETE` of a write) at most four sub-requests to a batch, Graph serving at most four concurrent requests per mailbox and throttling the others. A sub-request answered 429, 500, 502, 503 or 504 SHALL be sent again, up to three times, after the longest `Retry-After` its round asked for (1 second without one, 10 at most); one still failing SHALL keep its last status.

### Requirement: Graph folders are named by path
Graph SHALL list every mail folder but the hidden ones, walking the child folders of each folder holding some and following every paging link, a nested folder named by its path, the display names joined with `/` (`Projets/Alpha`). A mailbox value SHALL be resolved without listing the whole tree: a value shaped like a folder id (32 base64url characters or more) SHALL first be looked up with `GET /mailFolders/{value}`, which reaches a hidden folder; else, or when Graph knows no folder by it, the folders SHALL be walked as `mailbox list` walks them, only into a folder whose path followed by `/` begins the value, and the folder whose path is the value taken, several such folders being refused naming their ids; else a role name (`trash`, `sent`, compared ignoring case) SHALL be looked up as the well-known folder carrying it (`GET /mailFolders/deleteditems`), and a well-known name (`outbox`, `msgfolderroot`, compared ignoring case) as itself. The id a value resolves to SHALL be kept for the client's life, and forgotten once the client creates, renames or deletes a folder. Without a `mailbox.alias.trash`, the trash of `message delete` SHALL be the well-known `deleteditems` folder, asked alone. A value Graph answers 404 for, or 400 with the `error.code` `ErrorInvalidIdMalformed`, or any other value, SHALL be refused with ``Mailbox `X` not found``, coded `mailbox-not-found`, never sent into a request path; another failure of that lookup, another 400 included, SHALL fail the command. io-msgraph SHALL percent-encode every id it puts in a request path as one segment.

#### Scenario: A nested folder
- GIVEN a Graph account holding the folder `Alpha` under `Projets`
- WHEN `mailbox list` runs, then `envelope list -m Projets/Alpha`
- THEN the folder is listed as `Projets/Alpha`, and its envelopes are listed, the second command listing the top folders and the children of `Projets` alone to find it

### Requirement: Graph writes are scoped to the mailbox given
Graph message ids being account-wide and its message endpoints taking no folder, `flag add|set|remove`, `message copy`, `message move` and `message delete` SHALL first read the `parentFolderId` of each id (`GET /messages/{id}`, through `$batch`) and act only on the ids of the mailbox given. An id of another folder, one Graph answers 404 (unknown) or 400 `ErrorInvalidIdMalformed` (malformed) for, or one no path segment can carry SHALL count as not found; any other failed part of the lookup SHALL be an `errors` entry, as "Each backend names the items it could not serve" codes it. A move into its own source mailbox SHALL change nothing. `flag set` SHALL replace the categories of a message with the keyword flags it names, clearing them when it names none.

### Requirement: A Graph read is scoped to the mailbox given
The Graph backend SHALL read one message (`message read`, and the source of `message reply|forward` and `attachment list|download`) only when its `parentFolderId` is the folder the mailbox given resolves to, a well-known name included, read through `GET /messages/{id}?$select=id,parentFolderId` before `GET /messages/{id}/$value`, which carries no property: a message of another folder SHALL fail `message-not-found`, naming the mailbox, as on IMAP and JMAP, though a Graph id names a message account-wide, its bytes not fetched and `--seen` changing nothing. The mailbox SHALL be resolved ("Graph folders are named by path") only when it is not the message's folder id. An id Graph answers 404 or 400 `ErrorInvalidIdMalformed` for, at either request, SHALL fail `message-not-found`.

#### Scenario: An inbox message read under the sent mailbox
- GIVEN a Graph account whose message `m1` the Inbox holds
- WHEN `himalaya message read -m sentitems m1` runs
- THEN the command fails with the code `message-not-found`, naming `sentitems`, where it printed the message

### Requirement: Graph envelopes
An unpaged Graph `envelope list` or `envelope search` (`--page-size 0`) SHALL follow every paging link, asking 1000 messages a page, Graph's largest `$top`. A Graph envelope's date SHALL be its `sentDateTime`, the `Date:` header, else its `receivedDateTime`. Its `size` SHALL be `null`, Graph's message carrying no size of the raw message, as a JMAP or pimdir envelope whose size the server or the store does not give.

### Requirement: Mailbox role
A shared mailbox SHALL carry an optional role (inbox, all, archive, drafts, flagged, important, junk, sent, subscribed, trash, or a verbatim unknown one), shown by `mailbox list` in its text and JSON output. JMAP reads it from the mailbox `role`, Gmail from its fixed system-label ids, Microsoft Graph from its well-known folder names resolved to folder ids through `$batch` ("Graph batches four requests at a time"), a name Graph answers 404 for being skipped and any other failure failing the command, pimdir from the role its sync engine recorded from the server (`collections.role`), and IMAP from the special-use attributes its plain `LIST` carries besides `INBOX` (provider-quirks), as mbox marks its spool when one is configured and Maildir its inbox ("A Maildir is named relative to the root"), its only native role. On IMAP, JMAP, Gmail, Graph and pimdir a role name addresses the mailbox carrying it. A `mailbox.alias.<role>` entry overrides the native role of the mailbox it names, by id (on Maildir any spelling `-m` takes, a trailing slash or `./` included), else by the name of exactly one listed mailbox, on every backend, and `message delete` takes its trash from that alias before the backend's trash role; with neither, it SHALL fail with `capability-missing` before anything is moved. On IMAP, JMAP, Gmail and Graph a name several mailboxes carry SHALL be refused, naming their ids, and on JMAP and Gmail a value naming no mailbox by id, name or role SHALL be refused rather than sent to the server, as on Graph unless it may name a folder Graph does not list ("Graph folders are named by path").

### Requirement: Mailbox counts are asked for
`mailbox list` SHALL fill a mailbox's `total` and `unread` with `--counts` only, both `null` otherwise, Graph returning item counts inline included. With `--counts`, IMAP SHALL issue one `STATUS` per mailbox, JMAP read them from `Mailbox/get`, Gmail issue one `labels.get` per label, Graph count the folder's messages, mbox sync the index of each file, and pimdir count the items and the unread mail of the store as the overlaying reader lists them ("pimdir is a reader and a producer, never the owner"): the store's unread count (`count_unread`), which reads committed rows only, where io-pimdir answers that no pending action changes what the mailbox shows (`PimdirReader::overlay_quiet`), else the mailbox's items as listed, those without `\Seen` counted, so a staged flag change or move counts as `envelope list` shows it. Maildir SHALL leave both `null`. A Graph folder's `totalItemCount` counts its contacts and events too, so Graph SHALL count `total` through `$batch`, sent as "Graph batches four requests at a time" says, the `@odata.count` of the folder's `/messages` asked with `$count=true` over a one-message page, a folder whose item count is 0 printing that 0 unasked; `unread` SHALL be the folder's `unreadItemCount`, which counts its unread messages alone.

#### Scenario: A Graph folder holding contacts
- GIVEN a Graph Deleted Items folder answering `totalItemCount` 835 and `unreadItemCount` 5, whose `/messages` counts 26
- WHEN `mailbox list --counts` runs
- THEN it prints `total` 26 and `unread` 5, asking one count and none of the unread messages

#### Scenario: pimdir counts what it lists
- GIVEN a pimdir `INBOX` of 5 unread messages and an empty `Archive`, then `flag add seen 5` and `message move -t Archive 4` staged
- WHEN `mailbox list --counts` runs
- THEN `INBOX` prints `3/4` and `Archive` `1/1`, as `envelope list` shows them

### Requirement: Every backend builds alone
`cargo clippy --all-targets -- -D warnings` SHALL pass with the default features; with `--all-features`; with `--no-default-features` and a TLS provider alone (`rustls-ring`); with a TLS provider and each backend feature alone (`imap`, `smtp`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox`, `sieve`, `pimdir`); with `native-tls,jmap`; and with the three local backends together (`maildir,mbox,pimdir`). Code only some backends use SHALL be gated on the item with exactly those features, never allowed crate-wide. The configuration SHALL be a model holding no feature gate and no runtime code, compiled in every build, so every build parses and checks the same configuration, the blocks of the backends it leaves out included; it SHALL allow dead code once, at its root, in the builds missing a backend. Turning it into runtime values (TLS, proxy, SASL, server URL) SHALL belong to the features using them, gated with exactly those.

#### Scenario: A Maildir-only build
- GIVEN `--no-default-features --features rustls-ring,maildir`
- WHEN `cargo clippy --all-targets -- -D warnings` runs
- THEN it passes, where it reported the proxy, TLS, SASL and IMAP options of the configuration unread

#### Scenario: A block for a compiled-out backend
- GIVEN a Maildir-only build and an account whose `imap.sort` block carries an unknown key
- WHEN `account list` reads the configuration
- THEN it is refused naming that key, as a full build refuses it

### Requirement: Each backend names the items it could not serve
A backend SHALL report an item it could not list or act on as an `errors` entry, as precisely as its protocol tells, and SHALL never drop it with a warning alone. A JMAP listing paged client-side (a date clause on the chain of `and` from the root) SHALL report the emails that did not deserialize on its first page alone.

- IMAP: a FETCH line io-imap recovered in part SHALL list its message with the items that decoded and add a `message-unreadable` entry with its UID naming the failed items, `hasAttachment` `null` when `BODYSTRUCTURE` is one; a line whose `UID` is among the failed items SHALL leave its message out, its entry naming no id; a line io-imap skipped whole SHALL add an entry with a `null` id. A message a listing's window did not return SHALL be no error, the mailbox having changed since its `SELECT`. A UID given to a write and absent from both the `UID STORE` echoes and a `UID SEARCH` on the set SHALL be `message-not-found`. A refused `COPY` or `MOVE`, which answers for the whole set, SHALL stay a whole failure coded by its response code.
- JMAP: an email or a mailbox an `Email/query`, `Email/get` or `Mailbox/get` answered and that did not deserialize SHALL be `message-unreadable` or `mailbox-unreadable`, its id when the object names one; an id a write looked up whose email did not deserialize SHALL be `message-unreadable`. An id `Email/set` names in `notUpdated`, `notDestroyed` or `notCreated` SHALL take the code of its SetError type (`notFound` `message-not-found`, `forbidden` `permission-denied`, `overQuota` `quota-exceeded`, `rateLimit` `rate-limited`, any other `null`, its type in `sources`). An id the chained `Email/get` reports in `notFound` SHALL be left out of a listing.
- Gmail: a listing SHALL read its metadata through batched `messages.get`, sent as "Gmail batches fifty calls at a time" says; a part answered 404 SHALL be left out, a part still throttled after the retries SHALL be `rate-limited`, another 403 `permission-denied`, a 2xx part whose body does not parse `message-unreadable`, an unanswered part an entry coded `null`. A write SHALL look its ids up through batched `messages.get` (minimal), an id Gmail does not know (404, or 400 `Invalid id value`) or whose message lacks the mailbox's label being `message-not-found`, then flag, copy and move through batched `messages.modify` and delete through batched `messages.delete`, each message Gmail refuses an entry coded as a listing's part, a 404 `message-not-found`, the others written.
- Microsoft Graph: a write SHALL look its ids up and act on them through `$batch`, sent as "Graph batches four requests at a time" says; a part answered 404, or 400 with the `error.code` `ErrorInvalidIdMalformed`, SHALL be `message-not-found`, in the id lookup as in the write; the `error.code` `ErrorQuotaExceeded` or a 507 `quota-exceeded` whatever the status, then 429 after the retries `rate-limited` and 403 `permission-denied`; any other, another 400 included, `null` with Graph's `error.code` in `sources`; a 2xx lookup part whose body does not parse `message-unreadable`; a part the answer left out SHALL be an entry coded `null`. A listing, one request, SHALL fail whole.
- Maildir: a file that cannot be read, or holding no header field (no line opening on an RFC 5322 field name and a colon), SHALL be `message-unreadable` with its id, never a row, a folder that cannot be read `mailbox-unreadable` with its path, the id `mailbox list` gives it. `maildir list` SHALL warn of each folder it could not read.
- mbox: a message holding no header field SHALL be `message-unreadable` with its id, never a row.
- Maildir, mbox, pimdir: an unknown id SHALL be `message-not-found`. An mbox write, one rewrite under the lock, and a pimdir write, one queue transaction, SHALL stay whole failures. io-pimdir's write notes SHALL stay warnings.
- A mailbox whose counts could not be fetched (`--counts`) SHALL be `mailbox-unreadable`, its counts `null`.

A `message send` whose server refused a recipient (an SMTP `RCPT TO` refused, a JMAP `invalidRecipients`) SHALL fail whole, sent to nobody, coded `recipient-refused`: an SMTP send SHALL name the first refused address, the one io-smtp stopped at, and a JMAP send every address `invalidRecipients` lists.

#### Scenario: A Graph write with one part refused
- GIVEN a Graph folder holding messages `a` and `b`, the `$batch` answering the `PATCH` of `a` 200 and of `b` 403
- WHEN `himalaya --json flag add --flag seen a b` runs
- THEN it prints `count` 1 and `"errors":[{"id":"b","code":"permission-denied",…}]`, and the exit status is 3

### Requirement: Gmail batches fifty calls at a time
Gmail SHALL send its batched calls (the `messages.get` of a listing or of an id lookup, the `messages.modify` and `messages.delete` of a write) at most 50 to a batch, as Google advises for Gmail. A call answered 429, 403 for a rate limit (`rateLimitExceeded`, `userRateLimitExceeded`, `quotaExceeded`), 500, 502, 503 or 504 SHALL be sent again, up to three times, after the longest `Retry-After` its round asked for (1 second without one, 10 at most); one still failing SHALL keep its last answer. A call the batch answer left unanswered SHALL NOT be sent again.

#### Scenario: A throttled part
- GIVEN a write of 52 messages, Gmail answering the second call of the first batch 429 once
- WHEN the write runs
- THEN it sends a batch of 50 calls and one of 2, then the second call alone, and every message is written

### Requirement: A read asked to mark the message seen fails unmarked
`message read --seen` SHALL fail on every backend when the `\Seen` it asks for cannot be set: IMAP fetching without `PEEK`, Maildir and mbox renaming or rewriting the message, JMAP, Gmail and Graph sending the flag change, and pimdir staging it in its queue. The failure SHALL carry its cause's code (`unsupported` for a source of the pimdir store declaring `mail.flags.seen` `none`, `store-busy`, ...), and SHALL NOT be downgraded to a warning: a caller asking for the flag learns it is not set.

#### Scenario: A pimdir source that keeps no seen flag
- GIVEN a pimdir account whose source declares `mail.flags.seen` `none`
- WHEN `himalaya --json message read --seen <id>` runs
- THEN it fails coded `unsupported`, exit 1, and stages nothing

### Requirement: A read names its mailbox as a listing does
The read verbs (`message read`, and the source of `message reply|forward` and `attachment list|download`) SHALL take a mailbox by name, alias, role or id on every backend, as `envelope list` does: on IMAP a role (`sent`, `drafts`, `trash`, ...) SHALL select the mailbox whose special-use attribute names it, each adapter resolving the mailbox itself.

#### Scenario: A read of the sent mailbox on IMAP
- GIVEN an IMAP server listing `Sent Messages` with `\Sent`
- WHEN `himalaya message read -m sent 1` runs
- THEN it sends `SELECT "Sent Messages"`, as `envelope list -m sent` does

### Requirement: Addresses are read by one parser
An address header SHALL be read by mime-rs wherever Himalaya reads one from a header value: the Maildir, mbox and pimdir envelopes, the Gmail envelope from its metadata headers, the composers, and the read view, which drops comments and keeps an entry whose address is no valid one under `headers.invalid` (commands, "The read view lists unreadable address entries"). A comma or angle brackets inside a quoted display name SHALL stay in the name: `"Doe, John" <john@x.org>` is one address, named `Doe, John`.

### Requirement: imap thread fetches its envelopes in chunks
`imap thread` SHALL fetch the envelopes of the threaded messages 255 ids a `FETCH`, as io-imap's sort fallback chunks its own, so a set of scattered ids never makes a command line past what a server takes.

### Requirement: A Gmail label rename keeps the label
`gmail labels update <ID> <NAME>` SHALL rename the label through `users.labels.patch`, sending its name alone, so the label keeps its colour, its visibility and every other property, as the unified `mailbox rename` does.

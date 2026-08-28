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

Each backend SHALL source the field from the response its listing already makes, and SHALL leave it empty rather than issue a request of its own: IMAP from the `ENVELOPE` (RFC 3501 §7.4.2, 9th element), JMAP from the `inReplyTo` property of `Email/get`, Gmail from the metadata headers, Maildir and m2dir from the parsed message, and pimdir from the stored summary. Graph leaves it empty, `In-Reply-To` living in `internetMessageHeaders`, which a listing selection does not return.

The field SHALL NOT take a column in the `envelope list` table, where a column of raw msg-ids would be noise; it rides the JSON output.

### Requirement: Network backends
IMAP, JMAP, Gmail and Microsoft Graph SHALL each adapt their io-* high-level client. IMAP reuses io-imap's `select`/`fetch`/`store`/`copy`/`move`/`append`/`list`/`status`. JMAP reuses io-jmap's `mailbox_get`/`email_query`/`email_get`/`email_set`/`email_import`/`email_submission_set`/`blob_upload`/`blob_download`, addressing mailboxes by their JMAP id. Gmail treats labels as mailboxes over io-gmail's `labels`/`messages` surface; Graph treats mail folders as mailboxes over io-msgraph's `mail_folders`/`messages` surface.

### Requirement: Network transport resilience
The network backends SHALL run over a transport that retries a stream reporting it is not ready (`EAGAIN` on Unix, `EINTR`, and the Windows spelling of an expired deadline) rather than ending the exchange on it. Each read and each write carries its own budget of one minute, so a slow but progressing transfer never runs out of it, and exhausting the budget SHALL fail with a message naming it rather than a raw errno.

Opening a connection SHALL arm a socket read deadline matching that budget, so a server going silent on an otherwise healthy connection ends the command instead of blocking forever.

### Requirement: Local storage backends
Maildir, m2dir and pimdir SHALL adapt io-maildir, io-m2dir and io-pimdir. Maildir stores added messages under `cur/` and SHALL read an entry's flags through io-maildir rather than parsing the filename itself, so the meaning of a Maildir name is decided in the library that owns the format. m2dir is content-addressed with no native copy or move, so those are a get plus a store (plus a delete for move), and its flags live in a `.meta/<id>.flags` sidecar. m2dir mailbox `rename` and message `copy`/`move` remain unavailable until io-m2dir supports them. pimdir is an offline cache the sync engine (io-replica + io-pimdir) populates: reads project the store's shared items (io-pimdir's client read API) from the stored `v: 1` meta without body reads, and writes are staged io-replica `mutate` mutations a later sync propagates rather than direct SQL.

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

### Requirement: pimdir names a mailbox the way its server does
A hub collection is keyed `<namespace>/<name>`, so the pimdir backend SHALL show and accept the name without the namespace: the collection `imap/INBOX` is the mailbox `INBOX`. The namespace SHALL be derived when every mail collection of the account shares one prefix, which a single-source account always does; `pimdir.namespace` overrides it, and a store whose mail collections span two namespaces keeps whole ids as names rather than collapsing two mailboxes onto one.

A user-typed name SHALL resolve against the account's mail collections, a full collection id still being taken as itself. A name matching none, or several, SHALL be refused naming what the account holds. It SHALL NOT be passed to the store unresolved, which reads as a mailbox that exists and is empty.

### Requirement: pimdir reads one account
The pimdir backend SHALL show the collections of one account (pimdir SPEC §9.2), `pimdir.account` naming it. Unset, it is derived: a store holding one account, or one ungrouped set, is read as that one, and a store holding several is refused naming them rather than guessing one and showing the wrong mailbox set.

### Requirement: pimdir store path is shell-expanded
The pimdir backend SHALL expand `~` and environment variables on `pimdir.root` before opening the store and its blob reader, so a store path written with `~` (e.g. a Neverest store at `~/.local/state/neverest/<account>`) resolves to the home-relative directory. Opening the raw path would create an empty store at a literal `./~/…` and silently return an empty mailbox list.

### Requirement: pimdir is a reader and a producer, never the owner
The pimdir backend SHALL treat the store as a possibly-partial cache owned by the sync engine. `get_message` on an item whose body is not local (`level < Full`, no stored object) SHALL report a clear "body not fetched" state (the cue to sync), not a data-loss error; the item still lists.

Reads SHALL go through `PimdirReader`, the role that takes no lock (pimdir SPEC §8) and carries no write at all, so a sync in flight neither blocks Himalaya nor is blocked by it, and the backend cannot drain the queue or sweep the store even by mistake.

The reader SHALL overlay the queue (pimdir SPEC §15.4), so an action this client staged is visible on the next read rather than on the next sync: a staged `set-flags`, `update`, `remove`, `move` or `copy` changes what a listing shows. Each addresses a message that already exists and keeps its public id, so a staged write never changes how a message is addressed.

A write SHALL be staged as a queued `PimdirAction` through a producer handle (`store_flags`→`SetFlags`, `add_message`→`Add`, `copy_messages`→`Copy`, `move_messages`→`Move`, `delete_messages`→`Remove`), addressed by the public `seq`, for the store's owner to apply and push. The backend SHALL NOT write the index, load a collection, or run the owner's object sweep: a sweep run beside a sync destroys the bodies it has streamed but not yet attached, which SPEC §14 explicitly invites it to leave pending. A body an action references SHALL be written to the blob store durably before the action is enqueued, the queue row being what pins it.

`SetFlags` carries the whole replacement set, so applying it twice lands the same state; a set the store reports as unknown contributes no markers rather than staging an unknown one, which would erase what a sync knows. pimdir has no native trash.

An added message SHALL derive its link id, summary and sort key through `io_pimdir::conventions`, the one implementation of SPEC Annex A, which is the bare `Message-ID` with nothing prepended. A staged `Add` whose link id the collection already holds SHALL park (pimdir SPEC §15.3): it neither deduplicates against the stored copy nor mints a second key. Minting is the store's answer to what a source hands over; parking is its answer to a producer authoring a message the collection already has.

### Requirement: A queued creation is reported, not listed
A queued creation has no public id until the store's owner applies it, so the pimdir backend SHALL NOT project one as an envelope, and SHALL NOT put a placeholder in `Envelope.id`. `add_message` returns the link id it staged, which identifies the creation across the window.

An envelope listing SHALL report how many creations the mailbox has queued and name the command that shows them, so a saved message that is not in the list reads as queued rather than as lost. A backend that stages nothing reports none, which every backend whose writes reach the server as they are made does. An envelope *search* SHALL report none whatever the backend: a queued creation is never matched against the query, so a count its filter never saw would be misleading.

### Requirement: The pimdir subcommand reads and retracts the queue
Himalaya SHALL carry a `pimdir` subcommand for what the operator CLI cannot do without knowing mail. `queue list` SHALL render a queued creation as a message (flags, subject, recipient, and when it was queued, from the row's `created_at`) where the kind-agnostic `pimdir` binary can only print ids and hashes. `queue cancel` SHALL retract one row through io-pimdir's scoped owner operation, confirming first unless `--yes`.

Taking the owner role briefly is what cancelling costs (pimdir SPEC §15.5); the backend read and write paths SHALL NOT reach it. A store another process owns SHALL be refused immediately, saying a sync is running and that the action may already have been applied.

### Requirement: Append and search gaps
Gmail and Graph SHALL NOT implement `add_message` (neither API has an append) and SHALL NOT implement shared `search_envelopes`. IMAP, JMAP, Maildir and m2dir implement search (see the search capability).

### Requirement: Sending transport
Backends that self-send (JMAP, Gmail, Graph) SHALL route `send_message` through their own API. Storage backends that cannot send (IMAP, Maildir, m2dir) SHALL send through the account's SMTP transport, adapted in `src/smtp/backend.rs` over io-smtp, which parses the RFC 5321 envelope from the raw message headers.

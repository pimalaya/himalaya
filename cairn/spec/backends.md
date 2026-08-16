---
cairn: spec
capability: backends
status: current
---

# Backends

Each backend is a `<Proto>Client` wrapper that derefs onto the io-* `*Std` client, paired with a `src/<proto>/backend.rs` adapter implementing the shared operations over the client's high-level methods and converting io-* results into the CLI's own `email` shared types (`Address`, `Envelope`, `Flag`, `Mailbox`, and the search query). The CLI owns these types; no aggregator library sits between it and the io-* crates.

### Requirement: Shared operation set
The shared adapters SHALL cover, per backend: `list_mailboxes`, `list_envelopes`, `search_envelopes`, `store_flags`, `get_message`, `add_message`, `copy_messages`, `move_messages`, and `send_message`. A backend that cannot model an operation opts out of it rather than emulating it.

### Requirement: Network backends
IMAP, JMAP, Gmail and Microsoft Graph SHALL each adapt their io-* high-level client. IMAP reuses io-imap's `select`/`fetch`/`store`/`copy`/`move`/`append`/`list`/`status`. JMAP reuses io-jmap's `mailbox_get`/`email_query`/`email_get`/`email_set`/`email_import`/`email_submission_set`/`blob_upload`/`blob_download`, addressing mailboxes by their JMAP id. Gmail treats labels as mailboxes over io-gmail's `labels`/`messages` surface; Graph treats mail folders as mailboxes over io-msgraph's `mail_folders`/`messages` surface.

### Requirement: Network transport resilience
The network backends SHALL run over a transport that retries a stream reporting it is not ready (`EAGAIN` on Unix, `EINTR`, and the Windows spelling of an expired deadline) rather than ending the exchange on it. Each read and each write carries its own budget of one minute, so a slow but progressing transfer never runs out of it, and exhausting the budget SHALL fail with a message naming it rather than a raw errno.

Opening a connection SHALL arm a socket read deadline matching that budget, so a server going silent on an otherwise healthy connection ends the command instead of blocking forever.

### Requirement: Local storage backends
Maildir, m2dir and pimdir SHALL adapt io-maildir, io-m2dir and io-pimdir. Maildir stores added messages under `cur/` and SHALL read an entry's flags through io-maildir rather than parsing the filename itself, so the meaning of a Maildir name is decided in one place: the info section is delimited by the `:2,` marker, which matters because entry listing covers `new/` as well as `cur/` and a name in `new/` has no info section to read. m2dir is content-addressed with no native copy or move, so those are a get plus a store (plus a delete for move), and its flags live in a `.meta/<id>.flags` sidecar. m2dir mailbox `rename` and message `copy`/`move` remain unavailable until io-m2dir supports them. pimdir is an offline cache the sync engine (io-replica + io-pimdir) populates: reads project the store's shared items (io-pimdir's client read API) from the stored `v: 1` meta without body reads, and writes are staged io-replica `mutate` mutations a later sync propagates rather than direct SQL.

### Requirement: Maildir surfaces custom keywords on demand
The Maildir backend SHALL surface custom (non-IANA) keywords on read when told which convention the mailbox uses, so a keyword written by dovecot, mbsync, OfflineIMAP, mutt or notmuch matches a `flag <name>` search as it does on the network backends. `maildir.dovecot-keywords` SHALL resolve the lowercase info-section slot letters through the `dovecot-keywords` sidecar at the Maildir root, and `maildir.keywords-header` SHALL read keywords from `X-Keywords` (comma-separated) or `X-Label` (space-separated). Both default to off, and with both off the flag set SHALL be exactly the six standard info-section letters as before.

A sidecar that is absent, unreadable or disabled SHALL yield no keywords rather than fail the listing, since a mailbox without one is the normal case rather than an error.

### Requirement: pimdir shows a short public id
The pimdir backend SHALL show and accept each message's public id (`items.seq`, a small store-assigned integer, the same across every mailbox the message is filed in) as its `Envelope.id`, not the internal `link_id`. It SHALL resolve the id to the `link_id` (via the store's `get_item` / `seq_for_link`) before reading a body or staging an edit, and SHALL fail clearly on a non-numeric id. `add_message` SHALL return the new item's `seq`.

### Requirement: pimdir writes auto-source
When `pimdir.source` is unset, the pimdir backend SHALL attribute its writes to the store's single synced source (via `distinct_sources`) when there is exactly one — the local-sync case — so a staged edit propagates without configuration, falling back to `local` only when the store has no or several sources.

### Requirement: pimdir store path is shell-expanded
The pimdir backend SHALL expand `~` and environment variables on `pimdir.root` before opening the store and its blob reader, so a store path written with `~` (e.g. a Neverest store at `~/.local/state/neverest/<account>`) resolves to the home-relative directory. Opening the raw path would create an empty store at a literal `./~/…` and silently return an empty mailbox list.

### Requirement: pimdir is an availability-aware cache
The pimdir backend SHALL treat the store as a possibly-partial cache. `get_message` on an item whose body is not local (`level < Full`, no stored object) SHALL report a clear "body not fetched" state (the cue to sync), not a data-loss error; the item still lists. A staged write (`store_flags`→`SetFlags`, `add_message`→`Add`, `copy_messages`→`Copy`, `move_messages`→`Move`, `delete_messages`→`Remove`) is attributed to the configured `pimdir.source`; on a store not synced as that source (no binding for the item) the write SHALL fail loudly rather than stage a change no sync will carry. pimdir has no native trash. `add_message` content-hashes the body with the same digest as Neverest so an added message deduplicates against a synced one.

### Requirement: Append and search gaps
Gmail and Graph SHALL NOT implement `add_message` (neither API has an append) and SHALL NOT implement shared `search_envelopes`. IMAP, JMAP, Maildir and m2dir implement search (see the search capability).

### Requirement: Sending transport
Backends that self-send (JMAP, Gmail, Graph) SHALL route `send_message` through their own API. Storage backends that cannot send (IMAP, Maildir, m2dir) SHALL send through the account's SMTP transport, adapted in `src/smtp/backend.rs` over io-smtp, which parses the RFC 5321 envelope from the raw message headers.

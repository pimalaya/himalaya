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

### Requirement: Local storage backends
Maildir and m2dir SHALL adapt io-maildir and io-m2dir. Maildir stores added messages under `cur/`. m2dir is content-addressed with no native copy or move, so those are a get plus a store (plus a delete for move), and its flags live in a `.meta/<id>.flags` sidecar. m2dir mailbox `rename` and message `copy`/`move` remain unavailable until io-m2dir supports them.

### Requirement: Append and search gaps
Gmail and Graph SHALL NOT implement `add_message` (neither API has an append) and SHALL NOT implement shared `search_envelopes`. IMAP, JMAP, Maildir and m2dir implement search (see the search capability).

### Requirement: Sending transport
Backends that self-send (JMAP, Gmail, Graph) SHALL route `send_message` through their own API. Storage backends that cannot send (IMAP, Maildir, m2dir) SHALL send through the account's SMTP transport, adapted in `src/smtp/backend.rs` over io-smtp, which parses the RFC 5321 envelope from the raw message headers.

# Removing io-email from the CLI

Living plan for dropping the `io-email` dependency in favour of a local, lean, per-backend dispatching client owned by the CLI. Part of the wider aggregator retirement (`io-email`/`io-addressbook`/`io-calendar` are frozen and going protocol-direct or product-core).

## Why

`io-email` is a ~16k-line aggregation layer wrapping the per-protocol `io-*` crates behind a unified `EmailClientStd` dispatcher plus shared domain types. The CLI already drives every `io-*` crate directly for its protocol-specific subcommands (`himalaya imap …`, `himalaya jmap …`); `io-email` only supplied the *shared* cross-protocol layer used by the shared subcommands (`mailboxes`, `envelopes`, `flags`, `messages`, `attachments`). Maintaining a separate aggregator crate for that is a burden, so the CLI owns it instead. Duplication with `himalaya-tui` is acceptable: the two may diverge as their needs differ.

## Approach

Mirror cardamum's structure (`cardamum/src/shared/client.rs`): a local `EmailClient` holding one `BackendClient` enum variant per compiled-in backend, where each shared method matches on the active backend and dispatches to its adapter. The per-backend glue lives in each protocol module's `backend` submodule and reuses the existing protocol client, converting `io-*` results into the CLI's own shared types.

Unlike cardamum (contacts, one backend per account), mail also needs a send transport: the client keeps the storage backend plus an optional SMTP slot, so `send_message` works for IMAP/Maildir/m2dir accounts (JMAP/Gmail/Graph send through their own backend).

## Target architecture

```
src/email/                 shared domain types (LCD, serde-rendered)
  address.rs   Address
  envelope.rs  Envelope, normalize_message_id
  flag.rs      Flag, IanaFlag, FlagOp, classify_iana
  mailbox.rs   Mailbox, MailboxRole
  search.rs    SearchEmailsQuery + filter/sort + parser (TODO)

src/<proto>/backend.rs     thin shared-API adapter on the existing client
  impl <Proto>Client {      (crate::<proto>::client, which already wraps the
    fn list_envelopes(...)   io-* HIGH-LEVEL client: io_imap/io_maildir/... all
    ...                      expose list/select/fetch/store/copy/move/append)
  }                          shared input -> existing high-level calls ->
                             convert native results to crate::email types.
                             The only new code is the conversion, lifted from
                             io-email's <proto>/convert.rs + <op>/<proto> logic.

src/shared/client.rs       EmailClient { inner: BackendClient, smtp, account }
  enum BackendClient { Imap|Jmap|Gmail|Msgraph|Maildir|M2dir(Box<…Client>) }
  each shared method: match &mut self.inner { … }  (+ smtp for send)
```

Key point (confirmed while building the IMAP adapter): there is NO separate "backend driver". io-email shipped its own per-protocol client + coroutine layer only because it sat directly on the io-* low-level coroutines. Himalaya's protocol clients already wrap the io-* HIGH-LEVEL clients, so each adapter is just `impl <Proto>Client` with shared-typed methods calling those high-level methods and converting. io_imap, io_maildir and io_m2dir all expose rich high-level clients; the adapter never re-hosts coroutines.

## Operations to port

The shared subcommands call these client methods (each returns/consumes `crate::email` types):

- `list_mailboxes(with_counts) -> Vec<Mailbox>`
- `list_envelopes(mailbox, page, page_size, with_attachment) -> Vec<Envelope>`
- `search_envelopes(mailbox, query, page, page_size, with_attachment) -> Vec<Envelope>`
- `store_flags(mailbox, ids, flags, op) -> ()`
- `get_message(mailbox, id) -> Vec<u8>`
- `add_message(mailbox, flags, raw) -> String`
- `copy_messages(from, to, ids) -> ()`
- `move_messages(from, to, ids) -> ()`
- `send_message(raw) -> ()` (JMAP/Gmail/Graph backend, else SMTP)

Backends: `imap`, `jmap`, `gmail`, `msgraph`, `maildir`, `m2dir` (storage) plus `smtp` (send). Each `(operation, backend)` cell ports the matching io-email driver: `io-email/src/<domain>/<proto>/<op>.rs` (e.g. `envelope/imap/list.rs`, `flag/jmap/store.rs`, `message/smtp/send.rs`) plus the per-backend `io-email/src/<proto>/convert.rs`. Not every backend implements every operation (Gmail/Graph have no `add_message`; only JMAP/Gmail/Graph/SMTP send); the dispatcher bails with a clear message for unsupported cells, matching io-email's `UnsupportedOperation`.

## Flip strategy (keep the tree green)

The change is atomic: the current shared handlers receive io-email's `Envelope`/`Mailbox`/`Flag` from the io-email client, so type imports cannot flip until the client returns the local types. So build the replacement alongside io-email (it stays a dep, tree stays green), then swap in one step:

1. Inline shared types under `src/email/` (compile as additive modules). DONE for address/envelope/mailbox/flag; TODO for search.
2. Add `src/<proto>/backend.rs` adapters, one backend at a time, each compiling green (dead code until wired). Reuse the existing `crate::<proto>::client`; port conversion from io-email.
3. Rewrite `src/shared/client.rs` `EmailClient` to the `BackendClient` enum + SMTP slot, dispatching to the adapters.
4. Repoint the shared handlers' type imports `io_email::…` -> `crate::email::…` (see touch-points below), and drop the module-wide `#![allow(dead_code)]` in `src/email/mod.rs`.
5. Remove `io-email` from `Cargo.toml` (dependency, feature rows, and the `[patch.crates-io]` entry). `cargo build --all-features`, `cargo test`, `cargo clippy`, `cargo fmt`.

## Remaining io_email touch-points (as of this plan)

Client construction / wrappers:

- `src/shared/client.rs`: `io_email::client::EmailClientStd`, `io_email::{imap,maildir,m2dir,smtp}::client::*` (the whole file is rewritten in step 3).

Shared type imports to repoint in step 4:

- `src/shared/envelope/list.rs`: `Address`, `Envelope`, `Flag`
- `src/shared/envelope/search.rs`: `SearchEmailsQuery`, search `Error`
- `src/shared/mailbox/list.rs`: `Mailbox`
- `src/shared/flag/arg.rs`: `Flag`, `IanaFlag` (`impl From<&FlagArg>`)
- `src/shared/flag/{add,set,remove}.rs`: `Flag`, `FlagOp`
- `src/shared/message/add.rs`: `Flag`
- `src/shared/message/handler.rs`: `Flag`, `IanaFlag`

`src/gmail/client.rs` and `src/msgraph/client.rs` mention io-email in comments only; no code change needed there.

## Landed

- Inlined the shared domain types into `src/email/` (`address`, `envelope`, `flag`, `mailbox`), faithful to io-email minus the sync-only `EnvelopeDiff`/`MailboxDiff`/`FlagUpdate` the CLI does not use.
- Repointed the one client-independent use, `MailboxRole` in `src/imap/mailbox/list.rs`, to `crate::email::mailbox::MailboxRole`.
- IMAP adapter `src/imap/backend.rs`: `impl ImapClient` with the 7 non-search shared ops (`list_mailboxes`, `list_envelopes`, `store_flags`, `get_message`, `add_message`, `copy_messages`, `move_messages`), reusing io_imap's high-level `select`/`fetch`/`store`/`copy`/`move`/`append`/`list`/`status`, with the envelope/flag/mailbox conversion lifted from io-email. Compiles green as dead code (module-wide `#![allow(dead_code)]`). `search_envelopes` deferred until the shared search-query type is inlined. `add_message` drops io-email's synthetic-Message-ID injection (avoids a `uuid` dep): UIDPLUS first, else UID SEARCH on the message's own Message-ID, else bail.
- Maildir adapter `src/maildir/backend.rs`: same 7 ops, reusing io_maildir's high-level `list_maildirs`/`list_entries`/`read_entries`/`add_flags`/`set_flags`/`remove_flags`/`get`/`store`/`copy`/`move` via himalaya's `MaildirClient` (+ its `resolve_maildir`). Envelope conversion parses headers with `mail_parser`; flags read from the filename info section. `add_message` stores under `cur/`.
- m2dir adapter `src/m2dir/backend.rs`: same 7 ops, reusing io_m2dir's high-level `list_m2dirs`/`open_m2dir`/`list_entries`/`read_entry`/`read_flags`/`get`/`store`/`delete_entry`/`add_flags`/`set_flags`/`remove_flags`. m2dir is content-addressed with no native copy/move, so those are get + store (+ delete); flags live in the `.meta/<id>.flags` sidecar (`read_flags` per entry). `add_message` stores then writes flags.
- JMAP adapter `src/jmap/backend.rs`: the 7 ops PLUS `send_message`, reusing io_jmap's high-level `mailbox_get`/`email_query`/`email_get`/`email_set`/`email_import`/`email_submission_set`/`blob_upload`/`blob_download`. The shared `mailbox` arg is the JMAP mailbox id directly (no name resolution: the shared alias layer already yields it). `store_flags`/`copy`/`move` build one `Email/set` via `JmapEmailSetArgs`; `get_message` is `Email/get`(blobId) + `Blob/download` (download-URL template from the session); `add_message` is `Blob/upload` + `Email/import`; `send_message` is upload + import into drafts (`$draft`) + `EmailSubmission/set` under `config.identity_id` / `config.drafts_mailbox_id`.
- Gmail adapter `src/gmail/backend.rs`: `list_mailboxes`/`list_envelopes`/`store_flags`/`get_message`/`copy_messages`/`move_messages`/`send_message` (NO `add_message`: Gmail has no append). Reuses io_gmail's `labels_list`/`label_get`/`messages_list`/`message_get`/`message_modify`/`message_send`. Labels are mailboxes; a subset of system labels (UNREAD/STARRED/IMPORTANT/DRAFT/SPAM) back the shared flags (`\Seen` = absence of UNREAD); `list_envelopes` walks the opaque page token then does one metadata `message_get` per id; `get_message` is format=RAW + `decode_raw`; `send_message` is `encode_raw` + `message_send`.
- Microsoft Graph adapter `src/msgraph/backend.rs`: same 7 ops minus `add_message`, reusing io_msgraph's `mail_folders_list`/`messages_list`/`message_get_raw`/`message_update`/`message_copy`/`message_move`/`send_mail_mime`. Folders are mailboxes (counts inline); flags map to scalar fields (`isRead`, follow-up flag, importance) + `categories` via a `MsgraphMessage` PATCH; `list_envelopes` uses `$top`/`$skip`/`$select`; `get_message` is `$value`; `send_message` is `send_mail_mime`.
- SMTP adapter `src/smtp/backend.rs`: send-only `impl SmtpClient { send_message }`. Parses the RFC 5321 envelope from the raw headers (From: reverse path, To:/Cc:/Bcc: forward paths) via `mail_parser`, then reuses `SmtpClient::send` (io_smtp high-level). The dispatcher registers this as the sending transport for IMAP/Maildir/m2dir accounts.
- Shared search-query type inlined into `src/email/search/` (`query`/`filter`/`sort`/`parser`/`error`), a faithful copy of io-email's search module with `crate::search`->`crate::email::search`, `crate::flag::types::Flag`->`crate::email::flag::Flag`, no `alloc`, the `grammar.abnf` include dropped, and a hand-written `Error` (no `thiserror` dep). Added `chumsky = "0.13"` (himalaya's `ariadne` renders its `Rich` errors). All 10 ported parser tests pass.
- `search_envelopes` on the four searchable backends. Shared client-side matcher `src/email/search/eval.rs` (gated to maildir/m2dir): `matches_filter(envelope, raw, filter)` + `sort_envelopes`, lifted from io-email. IMAP (`imap/backend.rs`): SELECT -> UID SORT (query translated to `SearchKey`/`SortCriterion`, `sort_fallback()` for the client-side fallback) -> paginate uids -> UID FETCH -> reorder. maildir/m2dir: list, then `eval` filter+sort+paginate (body clauses reuse the read bytes). JMAP (`jmap/backend.rs`): translate to `JmapFilter`/`JmapEmailComparator` AND-scoped to the mailbox id; date clauses over-approximate (`after` >= midnight) and are re-checked client-side (`sentAt` vs Graph's `receivedAt`), paginating after the trim. gmail/msgraph have no shared search. 21 email-module tests pass.

## Remaining (per-backend recipe)

Each remaining backend follows the IMAP pattern: a `src/<proto>/backend.rs` with `impl <Proto>Client` shared methods over the existing high-level client, converting to `crate::email` types.

COMPLETE. io-email is no longer a dependency; the CLI owns its shared types + a per-backend dispatching client.

## The flip (landed)

- Rewrote `src/shared/client.rs` to `EmailClient { storage: Option<BackendClient>, smtp: Option<SmtpClient> }`. `BackendClient` is one boxed per-protocol client (selected by the `Backend` flag in local-before-network priority, matching io-email's read order). Every shared method matches the active backend and calls its adapter; `send_message` routes through a self-sending storage backend (JMAP/Gmail/Graph) else the SMTP slot; `search_envelopes`/`add_message` bail for the backends that lack them (Gmail/Graph).
- Repointed the nine shared-handler type imports `io_email::…` -> `crate::email::…`.
- Removed all seven `<proto>/backend.rs` + `email/mod.rs` `#![allow(dead_code)]` scaffolds; trimmed the few genuinely-unused inlined items surfaced (obsolete `FlagArg::imap`/`jmap`, `Address::new`/`with_name`, and five unused `Flag` predicates).
- Removed `io-email` from `Cargo.toml` (dep + `io-email/*` feature rows + `[patch.crates-io]`).
- Two feature-fallout fixes that io-email had been masking: `chrono` needs its `serde` feature (the inlined `Envelope` derives it), and the `maildir` feature must pull `io-maildir/parser` (io-email/maildir used to bring it in transitively; default features exclude maildir, so only `--all-features` caught it).

Verified: default + `--all-features` build clean, both clippy passes clean, `cargo test` (38 tests) green, `cargo fmt` clean, no code-level `io_email` references remain.

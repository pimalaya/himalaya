//! # Error codes
//!
//! A stable, machine-readable name for the failures a caller is expected
//! to act on, printed as `code` beside the message under `--json`.
//!
//! The message stays for humans and may be reworded at any time; the
//! code is the contract. A failure with no code prints `code: null`. The
//! mechanism (`CodedError`, `code_of`, the report) lives in
//! `pimalaya_cli::error`; only the vocabulary is Himalaya's.
//!
//! A pimdir store failure takes the code io-pimdir gives it
//! (`PimdirError::code`), spelled the same, so a caller reading every
//! pimdir app reads one vocabulary.
//!
//! A command a server refused takes the refusal's code (`permission-denied`,
//! `quota-exceeded`, `rate-limited`), an account pimalaya-config could not
//! take an `account-*` code: [`code`] reads them off the chain, and
//! [`coded`] attaches it at every exit (one-shot run, REPL line, MCP call).
//!
//! The same codes type an `ItemError`, the failure of one item a command
//! listing or acting on several could not serve, printed in its `errors`.

use anyhow::Error;
#[cfg(feature = "pimdir")]
use io_pimdir::{client::PimdirError, version::PimdirFormatRefusal};
use pimalaya_cli::error::{CodedError, code_of, with_code};
use pimalaya_config::toml::AccountError;
use schemars::JsonSchema;
use serde::Serialize;

/// The stable codes, each serialised in kebab-case.
///
/// Declared in every build, the pimdir ones included, so the
/// `himalaya-error` schema a caller reads does not change with the
/// backends compiled in.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    /// The message is listed but its body is not local yet: a sync will
    /// bring it, so the caller waits rather than reports a loss.
    BodyPending,
    /// The message is nested too deep or holds too many parts or header
    /// lines to be read whole, so it is refused rather than read in part.
    MessageTooComplex,
    /// The command would read its input off stdin inside a session, where
    /// stdin carries the commands: pass it as an argument or a file path.
    StdinTaken,
    /// An attachment exceeds the server's `--max-attachment-size`, refused
    /// before being decoded or encoded: nothing was done.
    AttachmentTooLarge,
    /// An inline attachment is malformed (no file name, both or neither of
    /// `data` and `text`, bad base64 or MIME type): nothing was composed.
    AttachmentInvalid,
    /// The search query does not parse: nothing was searched.
    SearchInvalid,
    /// The search query holds a clause the backend cannot search:
    /// nothing was searched, no clause dropped.
    SearchUnsupported,
    /// The backend does not serve the verb or the option asked: nothing was
    /// done. The message names the capability and what serves it instead.
    CapabilityMissing,
    /// The mailbox named does not exist: nothing was done.
    MailboxNotFound,
    /// The mailbox exists already (listed, queued for creation, or so the
    /// server answered): nothing was created nor staged.
    MailboxDuplicate,
    /// A message id given is not in the mailbox: an item error beside what
    /// was done, or the failure of a command reading that one message.
    MessageNotFound,
    /// The backend answered for a message the client could not read: an
    /// item error, or the failure of a command reading that one message.
    MessageUnreadable,
    /// A mailbox was listed but could not be read, its counts included.
    MailboxUnreadable,
    /// A JMAP thread id given is not one the server holds: an item error
    /// of `jmap thread get`.
    ThreadNotFound,
    /// A JMAP identity id given is not one the server holds: an item
    /// error of `jmap identity get`.
    IdentityNotFound,
    /// A JMAP email submission id given is not one the server holds: an
    /// item error of `jmap submission get` and `query`.
    SubmissionNotFound,
    /// The server refused the item, or the command, to the account.
    PermissionDenied,
    /// The write would exceed the account's quota.
    QuotaExceeded,
    /// The server still throttled the item after the retries.
    RateLimited,
    /// The server refused a recipient, so the message reached nobody.
    RecipientRefused,
    /// The pimdir store has another format version: upgrade Himalaya, or
    /// sync the store out with its engine and set it aside.
    StoreVersion,
    /// The pimdir store has no schema yet: its sync engine creates it.
    StoreUncreated,
    /// The database at the pimdir root is not a pimdir store: another
    /// program wrote it.
    StoreForeign,
    /// The pimdir store's version records disagree, or its schema is not
    /// the one its draft declares.
    StoreCorrupt,
    /// The pimdir store holds no schema yet, and the verb creates none:
    /// its sync engine creates it.
    StoreNotCurrent,
    /// The pimdir store names its bodies with a hash Himalaya does not
    /// compute.
    StoreHashAlgo,
    /// The pimdir store's sync state was written by another version of
    /// its sync engine.
    SyncVersion,
    /// Another writer holds the pimdir write lock, or an mbox dotlock or
    /// fcntl lock past `mbox.lock.timeout`. Retry.
    StoreBusy,
    /// A sync owns the pimdir store: retry once it ends.
    StoreOwned,
    /// A producer is staging a body in the pimdir store: retry.
    StoreStaging,
    /// A pimdir store verb is between two of its chunks: retry.
    StoreInFlight,
    /// The pimdir store is a link, or another user can read or write it:
    /// restrict it to its user.
    StoreExposed,
    /// The pimdir store is not the one that answered the queue row's
    /// handle, a copy restored since: nothing was done with the row.
    StoreReset,
    /// The lock file of the pimdir store was removed or replaced under a
    /// write: nothing was written. Retry.
    LockReplaced,
    /// The mailbox already holds the message, or has its creation
    /// queued: nothing was staged.
    ItemDuplicate,
    /// The pimdir store cannot hold the flag: empty, or holding a space, a
    /// control or a non-ASCII character. Nothing was staged.
    FlagInvalid,
    /// The pimdir action's payload is malformed: nothing was staged.
    ActionMalformed,
    /// The pimdir conflict holds a body nobody has read: fetch it first.
    ConflictUnread,
    /// A source of the pimdir store does not support the write: nothing
    /// was staged.
    Unsupported,
    /// No source of the pimdir store can perform the intent.
    NoPerformer,
    /// Several sources of the pimdir store can perform the intent and
    /// none is chosen.
    AmbiguousPerformer,
    /// The pimdir store has no search index yet: its sync engine builds
    /// it.
    IndexMissing,
    /// The pimdir search index was built by another version: its sync
    /// engine rebuilds it.
    IndexOutdated,
    /// The pimdir search index is of another version or shape: its sync
    /// engine rebuilds one of an earlier version, and leaves any other.
    IndexVersion,
    /// The zone the pimdir search index reads floating times in is not
    /// an IANA zone.
    IndexZoneUnknown,
    /// The SQLite Himalaya links lacks what the pimdir search index
    /// needs (FTS5, 3.43 or later).
    IndexFts5Missing,
    /// The pimdir search query is malformed or names an unknown field or
    /// value: nothing was searched.
    QueryRefused,
    /// The pimdir search index could not answer an accepted query, an
    /// implementation fault.
    QueryUnanswerable,
    /// The pimdir tag names no flag, or adds and removes one flag:
    /// nothing was staged.
    TagRefused,
    /// The pimdir store holds several accounts of mail and
    /// `pimdir.account` names none: nothing was read.
    StoreAccountAmbiguous,
    /// The pimdir store holds no collection of the account
    /// `pimdir.account` names: nothing was read.
    StoreAccountUnknown,
    /// No pimdir queue row of the account has the id given: nothing was
    /// cancelled.
    RowNotFound,
    /// The command asks to confirm, and no terminal can answer: pass
    /// `--yes`. Nothing was done.
    ConfirmationRequired,
    /// The page token is not one the command printed: nothing was listed.
    PageTokenInvalid,
    /// No account has the name `-a` gives (`default` among them, a name
    /// like any other).
    AccountMissing,
    /// No account is marked default, and `-a` names none.
    AccountDefaultMissing,
    /// Several accounts are marked default, and `-a` names none.
    AccountDefaultSeveral,
    /// The account's backend is one this build leaves out: rebuild with
    /// the cargo feature the message names.
    BackendNotBuilt,
}

impl ErrorCode {
    /// The code of an account the configuration could not give.
    pub fn of_account(err: &AccountError) -> Self {
        match err {
            AccountError::Missing { .. } => Self::AccountMissing,
            AccountError::NoDefault => Self::AccountDefaultMissing,
            AccountError::SeveralDefaults(_) => Self::AccountDefaultSeveral,
        }
    }
}

/// Gives a failure the code of the account or server refusal its chain
/// carries, its messages kept (see [`with_code`]).
pub fn coded(err: Error) -> Error {
    with_code(err, typed_code)
}

/// The code of `err`: the one a coded error in its chain carries, else
/// the one a typed error under it names.
pub fn code(err: &Error) -> Option<ErrorCode> {
    code_of(err).or_else(|| typed_code(err))
}

/// A reader of one kind of typed errors, coding those it knows.
type Reader = fn(&(dyn std::error::Error + 'static)) -> Option<ErrorCode>;

/// The code of the first cause in `err`'s chain a reader codes: the account
/// taken, or the refusal of the server that answered.
fn typed_code(err: &Error) -> Option<ErrorCode> {
    let readers: &[Reader] = &[
        account_code,
        #[cfg(feature = "imap")]
        crate::imap::client::refusal_code,
        #[cfg(feature = "jmap")]
        crate::jmap::error::refusal_code,
        #[cfg(feature = "gmail")]
        crate::gmail::client::refusal_code,
        #[cfg(feature = "msgraph")]
        crate::msgraph::client::refusal_code,
    ];

    err.chain()
        .find_map(|cause| readers.iter().find_map(|reader| reader(cause)))
}

/// The code of the account `-a` names, or the default one, not taken.
fn account_code(cause: &(dyn std::error::Error + 'static)) -> Option<ErrorCode> {
    cause
        .downcast_ref::<AccountError>()
        .map(ErrorCode::of_account)
}

/// A refusal under `mailbox-not-found`: the mailbox named does not
/// exist.
#[cfg_attr(
    not(any(
        feature = "imap",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph",
        feature = "maildir",
        feature = "mbox"
    )),
    allow(dead_code)
)]
pub fn mailbox_not_found(message: impl Into<String>) -> anyhow::Error {
    CodedError::new(ErrorCode::MailboxNotFound, message.into()).into()
}

/// The `mailbox-not-found` failure, worded `message`, the server's answer
/// `cause` kept in `sources`.
#[cfg(any(feature = "imap", feature = "msgraph"))]
pub fn unknown_mailbox(message: String, cause: impl Into<Error>) -> Error {
    with_code(cause.into(), |_| Some(ErrorCode::MailboxNotFound)).context(message)
}

/// The sentence of every refusal under `mailbox-duplicate`, the family's
/// one wording of a create naming a collection that exists.
fn mailbox_duplicate_sentence(name: &str) -> String {
    format!("Cannot create mailbox `{name}`: it exists already")
}

/// A refusal under `mailbox-duplicate`: the mailbox `name` a create names
/// exists already, as the local store holding it tells.
#[cfg_attr(not(feature = "maildir"), allow(dead_code))]
pub fn mailbox_duplicate(name: &str) -> Error {
    CodedError::new(
        ErrorCode::MailboxDuplicate,
        mailbox_duplicate_sentence(name),
    )
    .into()
}

/// The `mailbox-duplicate` failure of creating `name`, the backend's or
/// store's answer `cause` kept in `sources`.
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "mbox",
    feature = "pimdir"
))]
pub fn existing_mailbox(name: &str, cause: impl Into<Error>) -> Error {
    with_code(cause.into(), |_| Some(ErrorCode::MailboxDuplicate))
        .context(mailbox_duplicate_sentence(name))
}

/// A refusal under `message-not-found`: the message asked for is not in
/// the mailbox.
#[cfg_attr(
    not(any(
        feature = "imap",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph",
        feature = "maildir",
        feature = "mbox",
        feature = "pimdir"
    )),
    allow(dead_code)
)]
pub fn message_not_found(message: impl Into<String>) -> anyhow::Error {
    CodedError::new(ErrorCode::MessageNotFound, message.into()).into()
}

/// The `message-not-found` failure of reading `id`, the server's answer
/// `cause` kept in `sources`.
#[cfg(any(feature = "gmail", feature = "msgraph"))]
pub fn unknown_message(id: &str, cause: impl Into<Error>) -> Error {
    with_code(cause.into(), |_| Some(ErrorCode::MessageNotFound))
        .context(format!("Message `{id}` not found"))
}

/// The error of one item a command could not list or act on.
#[cfg(any(backend, feature = "smtp"))]
pub type ItemError = pimalaya_cli::error::ItemError<ErrorCode>;

/// The item error of the message `id` the mailbox does not hold.
#[cfg(any(backend, feature = "smtp"))]
#[cfg_attr(not(backend), allow(dead_code))]
pub fn missing_message(id: &str) -> ItemError {
    ItemError {
        id: Some(id.to_owned()),
        code: Some(ErrorCode::MessageNotFound),
        error: String::from("Message not found in the mailbox"),
        sources: Vec::new(),
    }
}

/// The item error of the message `id` a listing read but holding no
/// header field, so no envelope to list.
#[cfg(any(feature = "maildir", feature = "mbox"))]
pub fn headerless_message(id: &str) -> ItemError {
    ItemError {
        id: Some(id.to_owned()),
        code: Some(ErrorCode::MessageUnreadable),
        error: String::from("Cannot parse the message: it holds no header field"),
        sources: Vec::new(),
    }
}

/// Ends a command that printed its result: exit 3 when an item failed.
#[cfg(any(backend, feature = "smtp"))]
pub fn partial_when(errors: &[ItemError]) -> anyhow::Result<()> {
    match errors.is_empty() {
        true => Ok(()),
        false => Err(pimalaya_cli::error::PartialOutcome::default().into()),
    }
}

/// Writes the item errors after a command's output, one blank line and one
/// aligned `Error:` line each, nothing when there are none.
#[cfg(any(backend, feature = "smtp"))]
pub fn write_item_errors(
    f: &mut std::fmt::Formatter<'_>,
    errors: &[ItemError],
) -> std::fmt::Result {
    let mut lines = pimalaya_cli::error::ItemErrors::default();

    for error in errors {
        lines.push(error);
    }

    lines.write_after(f)
}

/// The refusal of a page token the command did not print, under
/// `page-token-invalid`.
pub fn page_token_invalid(token: &str) -> Error {
    let message = format!("Invalid page token `{token}`: pass a `nextPage` value as printed");
    CodedError::new(ErrorCode::PageTokenInvalid, message).into()
}

/// The pimdir codes Himalaya relays, by io-pimdir's spelling, which
/// serialises each.
#[cfg(feature = "pimdir")]
const PIMDIR_CODES: &[(&str, ErrorCode)] = &[
    ("store-version", ErrorCode::StoreVersion),
    ("store-uncreated", ErrorCode::StoreUncreated),
    ("store-foreign", ErrorCode::StoreForeign),
    ("store-corrupt", ErrorCode::StoreCorrupt),
    ("store-not-current", ErrorCode::StoreNotCurrent),
    ("store-hash-algo", ErrorCode::StoreHashAlgo),
    ("sync-version", ErrorCode::SyncVersion),
    ("store-busy", ErrorCode::StoreBusy),
    ("store-owned", ErrorCode::StoreOwned),
    ("store-staging", ErrorCode::StoreStaging),
    ("store-in-flight", ErrorCode::StoreInFlight),
    ("store-exposed", ErrorCode::StoreExposed),
    ("store-reset", ErrorCode::StoreReset),
    ("lock-replaced", ErrorCode::LockReplaced),
    ("item-duplicate", ErrorCode::ItemDuplicate),
    ("flag-invalid", ErrorCode::FlagInvalid),
    ("action-malformed", ErrorCode::ActionMalformed),
    ("conflict-unread", ErrorCode::ConflictUnread),
    ("unsupported", ErrorCode::Unsupported),
    ("no-performer", ErrorCode::NoPerformer),
    ("ambiguous-performer", ErrorCode::AmbiguousPerformer),
    ("index-missing", ErrorCode::IndexMissing),
    ("index-outdated", ErrorCode::IndexOutdated),
    ("index-version", ErrorCode::IndexVersion),
    ("index-zone-unknown", ErrorCode::IndexZoneUnknown),
    ("index-fts5-missing", ErrorCode::IndexFts5Missing),
    ("query-refused", ErrorCode::QueryRefused),
    ("query-unanswerable", ErrorCode::QueryUnanswerable),
    ("tag-refused", ErrorCode::TagRefused),
];

#[cfg(feature = "pimdir")]
impl ErrorCode {
    /// The code of a pimdir failure, `None` for one the caller cannot act
    /// on (an SQL, I/O or JSON failure).
    pub fn of_pimdir(err: &PimdirError) -> Option<Self> {
        let code = err.code();
        PIMDIR_CODES
            .iter()
            .find(|(pimdir, _)| *pimdir == code)
            .map(|(_, ours)| *ours)
    }
}

/// A pimdir failure as an error carrying its code, when it has one.
///
/// The message stays io-pimdir's, the caller adding its own context. A
/// store of another format version is refused in the wording every
/// pimdir program shows, [`PimdirFormatRefusal`], naming himalaya.
#[cfg(feature = "pimdir")]
pub fn pimdir_error(err: PimdirError) -> anyhow::Error {
    let message = match &err {
        PimdirError::Version { found } => PimdirFormatRefusal {
            found: found.as_deref(),
            own: io_pimdir::sql::FORMAT_VERSION,
            program: "himalaya",
        }
        .to_string(),
        err => err.to_string(),
    };

    match ErrorCode::of_pimdir(&err) {
        Some(code) => CodedError::new(code, message).into(),
        None => err.into(),
    }
}

#[cfg(all(test, feature = "pimdir"))]
mod tests {
    use super::*;

    #[test]
    fn a_relayed_pimdir_code_keeps_io_pimdirs_spelling() {
        for (pimdir, ours) in PIMDIR_CODES {
            assert_eq!(serde_json::to_value(ours).unwrap(), *pimdir);
        }
    }

    #[test]
    fn a_pimdir_failure_carries_its_code_under_context() {
        let err = pimdir_error(PimdirError::Busy).context("Stage add in `INBOX`");

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::StoreBusy)
        );
        assert!(format!("{err:#}").contains("write lock"), "{err:#}");
    }
}

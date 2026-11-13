//! # Capabilities
//!
//! What each backend serves of the unified API, and which configured
//! transport answers each unified verb for an account.
//!
//! The names and levels follow pimdir STORAGE Annex B without its domain
//! word: himalaya's `message.move` is pimdir's `mail.message.move`. A
//! backend declares a capability only when it gives the result every
//! other declarer gives; a different result stays protocol-specific.
//!
//! Two tiers decide. The static tier, the tables below, is known without
//! connecting, so a verb a backend cannot serve fails before credentials,
//! TLS or authentication. A row [`Support::Server`] leaves the answer to
//! what the server advertises once connected, the adapter refusing then.
//!
//! The tables are plain data compiled into every build, compiled-out
//! backends included, so the `--help` they annotate reads the same in
//! every build.

// NOTE: a build with no mail transport keeps the tables for the MCP
// server, which then has no verb to look them up for.
#![cfg_attr(not(any(backend, feature = "smtp")), allow(dead_code))]

use std::fmt;

use anyhow::Result;
use pimalaya_cli::error::CodedError;

use crate::{backend::Backend, config::AccountConfig, error::ErrorCode};

use Capability as C;
use SearchOption as O;
use Support::{Full, None as Not, Partial, Server};

/// A unified capability, named by [`Capability::name`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    /// Listing the mailboxes.
    MailboxList,
    /// Creating a mailbox, at the top or under another.
    MailboxCreate,
    /// Renaming a mailbox, its parent kept.
    MailboxRename,
    /// Deleting a mailbox and the messages it holds.
    MailboxDelete,
    /// Listing a mailbox's envelopes.
    EnvelopeList,
    /// Searching a mailbox with the shared query.
    EnvelopeSearch,
    /// One clause or sort key of the shared query.
    Search(SearchOption),
    /// Reading a message.
    MessageRead,
    /// Listing and downloading a message's attachments.
    AttachmentList,
    /// Adding a message to a mailbox.
    MessageAdd,
    /// Copying messages to another mailbox.
    MessageCopy,
    /// Moving messages to another mailbox.
    MessageMove,
    /// Removing messages for good.
    MessageRemove,
    /// Moving messages to the trash.
    MessageTrash,
    /// Adding and removing `\Seen`.
    FlagsSeen,
    /// Adding and removing `\Answered`.
    FlagsAnswered,
    /// Adding and removing `\Flagged`.
    FlagsFlagged,
    /// Adding and removing `\Draft`.
    FlagsDraft,
    /// Adding and removing `$Important` (RFC 8457).
    FlagsImportant,
    /// Adding and removing `$Junk`.
    FlagsJunk,
    /// Sending a message.
    Submit,
    /// Filing a copy of a sent message.
    SubmitCopy,
    /// Reading and setting the auto-reply.
    Vacation,
}

/// A clause or sort key of the shared search query, an option of
/// `envelope.search`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchOption {
    Date,
    After,
    Before,
    From,
    To,
    Cc,
    Bcc,
    Address,
    Subject,
    Body,
    Flag,
    Attachment,
    SortDate,
    SortFrom,
    SortTo,
    SortSubject,
}

impl SearchOption {
    /// The option as the query language spells it, `order by` keys
    /// prefixed `sort.`.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Date => "date",
            Self::After => "after",
            Self::Before => "before",
            Self::From => "from",
            Self::To => "to",
            Self::Cc => "cc",
            Self::Bcc => "bcc",
            Self::Address => "address",
            Self::Subject => "subject",
            Self::Body => "body",
            Self::Flag => "flag",
            Self::Attachment => "attachment",
            Self::SortDate => "sort.date",
            Self::SortFrom => "sort.from",
            Self::SortTo => "sort.to",
            Self::SortSubject => "sort.subject",
        }
    }
}

impl Capability {
    /// The capability's name, pimdir's grammar without the domain word.
    pub fn name(self) -> String {
        match self {
            Self::Search(option) => format!("envelope.search.{}", option.word()),
            Self::MailboxList => "mailbox.list".into(),
            Self::MailboxCreate => "mailbox.create".into(),
            Self::MailboxRename => "mailbox.rename".into(),
            Self::MailboxDelete => "mailbox.delete".into(),
            Self::EnvelopeList => "envelope.list".into(),
            Self::EnvelopeSearch => "envelope.search".into(),
            Self::MessageRead => "message.read".into(),
            Self::AttachmentList => "attachment.list".into(),
            Self::MessageAdd => "message.add".into(),
            Self::MessageCopy => "message.copy".into(),
            Self::MessageMove => "message.move".into(),
            Self::MessageRemove => "message.remove".into(),
            Self::MessageTrash => "message.trash".into(),
            Self::FlagsSeen => "flags.seen".into(),
            Self::FlagsAnswered => "flags.answered".into(),
            Self::FlagsFlagged => "flags.flagged".into(),
            Self::FlagsDraft => "flags.draft".into(),
            Self::FlagsImportant => "flags.important".into(),
            Self::FlagsJunk => "flags.junk".into(),
            Self::Submit => "submit".into(),
            Self::SubmitCopy => "submit.copy".into(),
            Self::Vacation => "vacation".into(),
        }
    }

    /// The pimdir STORAGE Annex B name of a write, the one a pimdir store's
    /// sources declare: the domain word back, `mailbox.` read as
    /// `collection.`, the trash a move. `None` for a read, which a store
    /// answers locally, and for an account feature, which it does not hold.
    #[cfg(feature = "pimdir")]
    pub fn annex_b_name(self) -> Option<&'static str> {
        let name = match self {
            Self::MailboxCreate => "collection.create",
            Self::MailboxRename => "collection.rename",
            Self::MailboxDelete => "collection.delete",
            Self::MessageAdd => "mail.message.add",
            Self::MessageCopy => "mail.message.copy",
            Self::MessageMove | Self::MessageTrash => "mail.message.move",
            Self::MessageRemove => "mail.message.remove",
            Self::FlagsSeen => "mail.flags.seen",
            Self::FlagsAnswered => "mail.flags.answered",
            Self::FlagsFlagged => "mail.flags.flagged",
            Self::FlagsDraft => "mail.flags.draft",
            Self::FlagsImportant => "mail.flags.important",
            Self::FlagsJunk => "mail.flags.junk",
            Self::Submit => "mail.submit",
            Self::SubmitCopy => "mail.submit.copy",
            Self::MailboxList
            | Self::EnvelopeList
            | Self::EnvelopeSearch
            | Self::Search(_)
            | Self::MessageRead
            | Self::AttachmentList
            | Self::Vacation => return None,
        };

        Some(name)
    }

    /// Whether the sending transport answers it rather than the storage
    /// backend.
    fn sends(self) -> bool {
        matches!(self, Self::Submit | Self::SubmitCopy)
    }

    /// Whether it is a feature of the account rather than of a message,
    /// which the first configured backend serving it answers, whatever the
    /// storage backend.
    fn is_account_feature(self) -> bool {
        matches!(self, Self::Vacation)
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name())
    }
}

/// How well a backend serves a capability, pimdir's levels plus the one
/// the server decides.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Support {
    /// Served with the result every declarer gives.
    Full,
    /// Served, the detail saying what differs or is missing.
    Partial(&'static str),
    /// Served when the server advertises what it needs, the detail
    /// saying what; refused once connected otherwise.
    Server(&'static str),
    /// Not served, the detail saying why and what serves it instead.
    None(&'static str),
}

impl Support {
    /// The level's name, as the help prints it.
    pub fn level(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Partial(_) => "partial",
            Self::Server(_) => "server",
            Self::None(_) => "none",
        }
    }

    /// What the level leaves out, `None` for a full one.
    pub fn detail(self) -> Option<&'static str> {
        match self {
            Self::Full => None,
            Self::Partial(detail) | Self::Server(detail) | Self::None(detail) => Some(detail),
        }
    }

    /// Whether the capability is served at all, maybe in part.
    pub fn is_served(self) -> bool {
        !matches!(self, Self::None(_))
    }
}

/// How a backend serves a capability as read at run time, its detail
/// owned: what a pimdir store's sources declare, which its static row
/// leaves to the store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoreSupport {
    /// The level, as [`Support::level`] spells it.
    pub level: &'static str,
    /// What the level leaves out, `None` for a full one.
    pub detail: Option<String>,
}

impl From<Support> for StoreSupport {
    fn from(support: Support) -> Self {
        Self {
            level: support.level(),
            detail: support.detail().map(ToString::to_string),
        }
    }
}

/// One declared capability of a backend.
type Row = (Capability, Support);

/// What a backend declares, nothing for the ones serving no unified verb.
pub fn declared(backend: Backend) -> &'static [Row] {
    match backend {
        Backend::Imap => IMAP,
        Backend::Jmap => JMAP,
        Backend::Gmail => GMAIL,
        Backend::Msgraph => MSGRAPH,
        Backend::Maildir => MAILDIR,
        Backend::Mbox => MBOX,
        Backend::Pimdir => PIMDIR,
        Backend::Smtp => SMTP,
        Backend::Auto | Backend::Sieve => &[],
    }
}

/// How `backend` serves `capability`, `none` for a row it does not
/// declare.
pub fn support(backend: Backend, capability: Capability) -> Support {
    declared(backend)
        .iter()
        .find(|(declared, _)| *declared == capability)
        .map(|(_, support)| *support)
        .unwrap_or(Support::None("not implemented"))
}

/// The storage backends, in the order an account's first configured
/// one is picked: local before network.
pub const STORAGE_ORDER: &[Backend] = &[
    Backend::Maildir,
    Backend::Mbox,
    Backend::Pimdir,
    Backend::Jmap,
    Backend::Gmail,
    Backend::Msgraph,
    Backend::Imap,
];

/// The storage backends that send mail themselves, the others sending
/// over SMTP.
const SENDING_STORAGE: &[Backend] = &[
    Backend::Jmap,
    Backend::Gmail,
    Backend::Msgraph,
    Backend::Pimdir,
];

/// The transports an account's unified verbs resolve to.
///
/// The verbs exchanging ids (mailbox, envelope, message, flag,
/// attachment) all resolve to the one storage backend, so an id one
/// returns is valid in the others; sending resolves on its own.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolution {
    /// The account's name, for the errors.
    pub account: String,
    /// The `--backend` filter it was resolved under.
    pub filter: Backend,
    /// The storage backend, the first of [`STORAGE_ORDER`] configured and
    /// allowed.
    pub storage: Option<Backend>,
    /// The sending transport: the storage backend when it sends, else
    /// SMTP when configured and allowed.
    pub sender: Option<Backend>,
    /// The backend answering the account's features: the first of
    /// [`STORAGE_ORDER`] configured and allowed that serves them, else the
    /// storage backend, whose refusal then says what serves them instead.
    pub features: Option<Backend>,
    /// Whether a Maildir keeps keywords, in its dovecot-keywords file:
    /// `maildir.keywords.dovecot`, the header mode reading only.
    pub maildir_keywords: bool,
}

impl Resolution {
    /// Resolves an account's transports, reading only which blocks it
    /// configures, so nothing connects.
    pub fn of(account: &str, config: &AccountConfig, filter: Backend) -> Self {
        let storage = STORAGE_ORDER
            .iter()
            .copied()
            .find(|backend| allows(filter, *backend) && configures(config, *backend));

        let sender = match storage {
            Some(storage) if SENDING_STORAGE.contains(&storage) => Some(storage),
            _ if allows(filter, Backend::Smtp) && configures(config, Backend::Smtp) => {
                Some(Backend::Smtp)
            }
            _ => None,
        };

        // NOTE: an account syncing a local store from its provider keeps
        // its auto-reply on the provider, which the storage order would
        // never reach.
        let features = STORAGE_ORDER
            .iter()
            .copied()
            .find(|backend| {
                allows(filter, *backend)
                    && configures(config, *backend)
                    && support(*backend, Capability::Vacation).is_served()
            })
            .or(storage);

        #[cfg(feature = "maildir")]
        let maildir_keywords = config
            .maildir
            .as_ref()
            .is_some_and(|maildir| maildir.keywords.dovecot);
        #[cfg(not(feature = "maildir"))]
        let maildir_keywords = false;

        Self {
            account: account.to_string(),
            filter,
            storage,
            sender,
            features,
            maildir_keywords,
        }
    }

    /// The transport answering `capability`, `None` when none is
    /// configured.
    pub fn transport(&self, capability: Capability) -> Option<Backend> {
        if capability.sends() {
            self.sender
        } else if capability.is_account_feature() {
            self.features
        } else {
            self.storage
        }
    }

    /// How the account serves `capability`, `none` when no transport
    /// answers it.
    pub fn support(&self, capability: Capability) -> Support {
        match self.transport(capability) {
            Some(backend) => self.backend_support(backend, capability),
            None => Support::None("no transport"),
        }
    }

    /// Refuses the first of `capabilities` the account does not serve.
    ///
    /// A search option is refused as a search, under `search-unsupported`,
    /// any other capability under `capability-missing`.
    pub fn require(&self, capabilities: &[Capability]) -> Result<()> {
        for capability in capabilities {
            let Some(backend) = self.transport(*capability) else {
                return Err(self.no_transport(*capability));
            };

            let Support::None(detail) = self.backend_support(backend, *capability) else {
                continue;
            };

            if let Capability::Search(option) = capability {
                let word = option.word();
                let message = format!("Cannot search `{word}` over {backend}: {detail}");
                return Err(CodedError::new(ErrorCode::SearchUnsupported, message).into());
            }

            let account = &self.account;
            let message =
                format!("Account `{account}` cannot `{capability}` over {backend}: {detail}");
            return Err(missing(message));
        }

        Ok(())
    }

    /// How `backend` serves `capability` for this account: its row, the
    /// Maildir keyword rows read from the account's configuration.
    fn backend_support(&self, backend: Backend, capability: Capability) -> Support {
        let keyword = matches!(
            capability,
            Capability::FlagsImportant | Capability::FlagsJunk
        );
        if backend == Backend::Maildir && keyword && !self.maildir_keywords {
            return Support::None(MAILDIR_NO_KEYWORD);
        }
        support(backend, capability)
    }

    /// The refusal of a capability no configured transport answers.
    fn no_transport(&self, capability: Capability) -> anyhow::Error {
        let account = &self.account;
        let kind = if capability.sends() {
            "sending backend (JMAP, Gmail, Graph, pimdir or SMTP)"
        } else if capability.is_account_feature() {
            "backend serving it"
        } else {
            "storage backend"
        };
        let filter = match self.filter {
            Backend::Auto => String::new(),
            filter => format!(" that `--backend {filter}` allows"),
        };

        missing(format!(
            "Account `{account}` cannot `{capability}`: it configures no {kind}{filter}"
        ))
    }
}

/// A refusal under `capability-missing`, raised by the resolver up front
/// or by an adapter once the server said what it lacks.
pub fn missing(message: impl Into<String>) -> anyhow::Error {
    CodedError::new(ErrorCode::CapabilityMissing, message.into()).into()
}

/// Whether `--backend` lets `backend` serve a unified verb.
fn allows(filter: Backend, backend: Backend) -> bool {
    filter == Backend::Auto || filter == backend
}

/// Whether the account configures `backend`, always `false` for one
/// this build compiles out.
#[cfg_attr(
    not(all(
        feature = "imap",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph",
        feature = "maildir",
        feature = "mbox",
        feature = "pimdir",
        feature = "smtp"
    )),
    allow(unused_variables)
)]
fn configures(config: &AccountConfig, backend: Backend) -> bool {
    match backend {
        #[cfg(feature = "imap")]
        Backend::Imap => config.imap.is_some(),
        #[cfg(feature = "jmap")]
        Backend::Jmap => config.jmap.is_some(),
        #[cfg(feature = "gmail")]
        Backend::Gmail => config.gmail.is_some(),
        #[cfg(feature = "msgraph")]
        Backend::Msgraph => config.msgraph.is_some(),
        #[cfg(feature = "maildir")]
        Backend::Maildir => config.maildir.is_some(),
        #[cfg(feature = "mbox")]
        Backend::Mbox => config.mbox.is_some(),
        #[cfg(feature = "pimdir")]
        Backend::Pimdir => config.pimdir.is_some(),
        #[cfg(feature = "smtp")]
        Backend::Smtp => config.smtp.is_some(),
        _ => false,
    }
}

/// What a copy is on the backends where it is one message in two
/// mailboxes.
const COPY_SHARED: &str = "the message is added to the target mailbox rather than duplicated, so \
     both share flags, and deleting one deletes both";

/// What the backends finding the trash by alias alone need.
const TRASH_ALIAS: &str = "needs `mailbox.alias.trash`, the backend marking no trash";

/// What deleting a mailbox holding others does on IMAP.
const KEEPS_INFERIORS: &str = "a mailbox holding others is refused by the servers keeping inferior names (RFC 3501 section 6.3.4)";

/// What deleting a mailbox holding others does in a local store.
const LOCAL_INFERIORS: &str =
    "a mailbox holding others is refused, naming them, rather than deleted with them";

/// What a page-local sort leaves out.
const PAGE_SORT: &str = "orders the fetched page only, the server answering in its own order";

/// What a keyword flag needs on IMAP.
const KEYWORD_PERMANENT: &str = "a keyword, kept only when the mailbox's `PERMANENTFLAGS` lists it or `\\*` (RFC 3501 section 7.1)";

/// What a keyword flag is in a Maildir, the header mode only reading.
const MAILDIR_KEYWORD: &str = "a keyword, kept only with `maildir.keywords.dovecot` on, in the dovecot-keywords file, which other readers may not see";

/// Why a Maildir account without dovecot keywords refuses a keyword flag.
const MAILDIR_NO_KEYWORD: &str =
    "a keyword is kept only in the dovecot-keywords file; set `maildir.keywords.dovecot = true`";

/// Why a mail store holds no auto-reply.
const NO_VACATION: &str = "the auto-reply lives on the provider; configure its JMAP or Gmail block";

/// The unified verbs every storage backend serves the same way.
macro_rules! storage_rows {
    ($($row:expr),* $(,)?) => {
        &[
            (C::MailboxList, Full),
            (C::EnvelopeList, Full),
            (C::MessageRead, Full),
            (C::AttachmentList, Full),
            (C::MessageMove, Full),
            (C::FlagsSeen, Full),
            (C::FlagsFlagged, Full),
            (C::Search(O::From), Full),
            (C::Search(O::To), Full),
            (C::Search(O::Cc), Full),
            (C::Search(O::Bcc), Full),
            (C::Search(O::Address), Full),
            (C::Search(O::Subject), Full),
            $($row),*
        ]
    };
}

/// IMAP (RFC 3501).
const IMAP: &[Row] = storage_rows![
    (
        C::EnvelopeSearch,
        Partial(
            "text as the server matches it: RFC 3501 asks for a case-insensitive substring, some servers (Cyrus, Fastmail) match whole words"
        )
    ),
    (C::MailboxCreate, Full),
    (C::MailboxRename, Full),
    (C::MailboxDelete, Partial(KEEPS_INFERIORS)),
    (C::FlagsImportant, Server(KEYWORD_PERMANENT)),
    (C::FlagsJunk, Server(KEYWORD_PERMANENT)),
    (
        C::Vacation,
        Not(
            "IMAP holds no auto-reply; `himalaya sieve` writes a vacation script (RFC 5230), which has no date window"
        )
    ),
    (C::MessageAdd, Full),
    (C::MessageCopy, Full),
    (
        C::MessageRemove,
        Server(
            "UIDPLUS (RFC 4315) removes them; without it they are flagged `\\Deleted`, the expunge left owing"
        )
    ),
    (
        C::MessageTrash,
        Server("`mailbox.alias.trash`, else the mailbox the server marks `\\Trash` (RFC 6154)")
    ),
    (C::FlagsAnswered, Full),
    (C::FlagsDraft, Full),
    (C::Search(O::Date), Full),
    (C::Search(O::After), Full),
    (C::Search(O::Before), Full),
    (C::Search(O::Body), Full),
    (C::Search(O::Flag), Full),
    (
        C::Search(O::Attachment),
        Not("IMAP SEARCH has no attachment key")
    ),
    (C::Search(O::SortDate), Full),
    (C::Search(O::SortFrom), Full),
    (C::Search(O::SortTo), Full),
    (C::Search(O::SortSubject), Full),
];

/// JMAP (RFC 8620, RFC 8621).
const JMAP: &[Row] = storage_rows![
    (C::MailboxCreate, Full),
    (C::MailboxRename, Full),
    (
        C::MailboxDelete,
        Partial(
            "a message also in another mailbox stays there, JMAP holding one message in several"
        )
    ),
    (C::FlagsImportant, Full),
    (C::FlagsJunk, Full),
    (
        C::Vacation,
        Server("needs `urn:ietf:params:jmap:vacationresponse` in the session (RFC 8621 section 8)")
    ),
    (
        C::EnvelopeSearch,
        Partial(
            "text matches as the server defines `text` and the header conditions (RFC 8621 section 4.4.1)"
        )
    ),
    (C::MessageAdd, Full),
    (C::MessageCopy, Partial(COPY_SHARED)),
    (C::MessageRemove, Full),
    (C::MessageTrash, Full),
    (C::FlagsAnswered, Full),
    (C::FlagsDraft, Full),
    (C::Search(O::Date), Partial(JMAP_DATE)),
    (C::Search(O::After), Partial(JMAP_DATE)),
    (C::Search(O::Before), Partial(JMAP_DATE)),
    (C::Search(O::Body), Full),
    (C::Search(O::Flag), Full),
    (C::Search(O::Attachment), Full),
    (C::Search(O::SortDate), Full),
    (C::Search(O::SortFrom), Full),
    (C::Search(O::SortTo), Full),
    (C::Search(O::SortSubject), Full),
    (
        C::Submit,
        Server("needs `urn:ietf:params:jmap:submission` in the session (RFC 8621 section 7)")
    ),
    (C::SubmitCopy, Full),
];

/// Where JMAP's date clauses differ.
const JMAP_DATE: &str = "under `or` or `not`, compared with the received date rather than `Date:`";

/// Gmail.
const GMAIL: &[Row] = storage_rows![
    (C::MailboxCreate, Full),
    (C::MailboxRename, Full),
    (
        C::MailboxDelete,
        Partial(
            "the label goes, its messages staying under their other labels or in All Mail and its sublabels under their full names"
        )
    ),
    (C::FlagsImportant, Full),
    (
        C::FlagsJunk,
        Partial("the `SPAM` label, which moves the message to Spam")
    ),
    (C::Vacation, Full),
    (
        C::EnvelopeSearch,
        Partial("text matches by words, as the Gmail search box does")
    ),
    (
        C::MessageAdd,
        Partial("inserted as `messages.insert` does: no filters, no spam scan")
    ),
    (C::MessageCopy, Partial(COPY_SHARED)),
    (C::MessageRemove, Full),
    (C::MessageTrash, Full),
    (C::FlagsAnswered, Not("Gmail has no label for it")),
    (
        C::FlagsDraft,
        Not(
            "Gmail applies `DRAFT` itself and refuses it from a client; `gmail drafts` makes a draft"
        )
    ),
    (C::Search(O::Date), Partial(GMAIL_DATE)),
    (C::Search(O::After), Partial(GMAIL_DATE)),
    (C::Search(O::Before), Partial(GMAIL_DATE)),
    (
        C::Search(O::Body),
        Partial("a word anywhere in the message, headers included")
    ),
    (
        C::Search(O::Flag),
        Partial("`seen`, `flagged`, `draft`, `important` and `junk` only")
    ),
    (
        C::Search(O::Attachment),
        Partial("Gmail's `has:attachment`, which counts inline images too")
    ),
    (C::Search(O::SortDate), Partial(PAGE_SORT)),
    (C::Search(O::SortFrom), Partial(PAGE_SORT)),
    (C::Search(O::SortTo), Partial(PAGE_SORT)),
    (C::Search(O::SortSubject), Partial(PAGE_SORT)),
    (C::Submit, Full),
    (C::SubmitCopy, Full),
];

/// Where Gmail's date clauses differ.
const GMAIL_DATE: &str = "compared with the received date, in UTC days, rather than `Date:`";

/// Microsoft Graph.
const MSGRAPH: &[Row] = storage_rows![
    (
        C::MailboxCreate,
        Partial("under `--parent`, created at the top then moved under it, two requests")
    ),
    (C::MailboxRename, Full),
    (
        C::MailboxDelete,
        Partial("the folders it holds are deleted with it")
    ),
    (
        C::FlagsImportant,
        Partial("`importance: high`, the sender's priority rather than the reader's mark")
    ),
    (
        C::FlagsJunk,
        Not("Graph has no junk mark; `message move` to the Junk Email folder")
    ),
    (
        C::Vacation,
        Not("Graph keeps it in `mailboxSettings`, which io-msgraph does not cover yet")
    ),
    (
        C::EnvelopeSearch,
        Partial("text matches by KQL words, and results come in relevance order")
    ),
    (
        C::MessageAdd,
        Not("Graph creates drafts only; `msgraph messages create` makes one")
    ),
    (C::MessageCopy, Full),
    (C::MessageRemove, Full),
    (C::MessageTrash, Full),
    (C::FlagsAnswered, Not("Graph has no field for it")),
    (C::FlagsDraft, Not("Graph's `isDraft` is read-only")),
    (C::Search(O::Date), Full),
    (C::Search(O::After), Full),
    (C::Search(O::Before), Full),
    (C::Search(O::Body), Full),
    (
        C::Search(O::Flag),
        Not("KQL has no flag property, and Graph takes no `$filter` beside `$search`")
    ),
    (C::Search(O::Attachment), Full),
    (C::Search(O::SortDate), Partial(PAGE_SORT)),
    (C::Search(O::SortFrom), Partial(PAGE_SORT)),
    (C::Search(O::SortTo), Partial(PAGE_SORT)),
    (C::Search(O::SortSubject), Partial(PAGE_SORT)),
    (C::Submit, Full),
    (
        C::SubmitCopy,
        Partial(
            "Graph files every message it sends in Sent Items itself: the save options of a send (`--save`, `--no-save`, `message.send.save-copy`) are ignored"
        )
    ),
];

/// The unified verbs the local backends serve alike, evaluating the query
/// themselves.
macro_rules! local_rows {
    ($($row:expr),* $(,)?) => {
        storage_rows![
            (C::EnvelopeSearch, Full),
            (C::MessageAdd, Full),
            (C::MessageCopy, Full),
            (C::MessageRemove, Full),
            (C::MessageTrash, Partial(TRASH_ALIAS)),
            (C::FlagsAnswered, Full),
            (C::FlagsDraft, Full),
            (C::Search(O::Date), Full),
            (C::Search(O::After), Full),
            (C::Search(O::Before), Full),
            (C::Search(O::Body), Full),
            (C::Search(O::Flag), Full),
            (C::Search(O::Attachment), Full),
            (C::Search(O::SortDate), Full),
            (C::Search(O::SortFrom), Full),
            (C::Search(O::SortTo), Full),
            (C::Search(O::SortSubject), Full),
            $($row),*
        ]
    };
}

/// Maildir.
const MAILDIR: &[Row] = local_rows![
    (C::MailboxCreate, Full),
    (C::MailboxRename, Full),
    (C::MailboxDelete, Partial(LOCAL_INFERIORS)),
    (C::FlagsImportant, Partial(MAILDIR_KEYWORD)),
    (C::FlagsJunk, Partial(MAILDIR_KEYWORD)),
    (C::Vacation, Not(NO_VACATION)),
];

/// mbox.
const MBOX: &[Row] = local_rows![
    (
        C::MailboxCreate,
        Partial(
            "a mailbox nests under an mbox file only with `mbox.thunderbird`; the plain layout refuses that `--parent`"
        )
    ),
    (C::MailboxRename, Full),
    (C::MailboxDelete, Full),
    (
        C::FlagsImportant,
        Partial("a keyword in `X-Keywords`, which other readers may not see")
    ),
    (
        C::FlagsJunk,
        Partial("a keyword in `X-Keywords`, which other readers may not see")
    ),
    (C::Vacation, Not(NO_VACATION)),
];

/// pimdir: the store answers reads locally and
/// queues writes for its sync engine, which refuses a write no source
/// performs with pimdir's own codes (`unsupported`, `no-performer`).
const PIMDIR: &[Row] = storage_rows![
    (C::MailboxCreate, Full),
    (C::MailboxRename, Full),
    (C::MailboxDelete, Full),
    (C::FlagsImportant, Full),
    (C::FlagsJunk, Full),
    (C::Vacation, Not(NO_VACATION)),
    (C::EnvelopeSearch, Full),
    (C::MessageAdd, Full),
    (C::MessageCopy, Full),
    (C::MessageRemove, Full),
    (C::MessageTrash, Full),
    (C::FlagsAnswered, Full),
    (C::FlagsDraft, Full),
    (C::Search(O::Date), Full),
    (C::Search(O::After), Full),
    (C::Search(O::Before), Full),
    (
        C::Search(O::Body),
        Partial("a message whose body is not local matches no body clause")
    ),
    (C::Search(O::Flag), Full),
    (C::Search(O::Attachment), Full),
    (C::Search(O::SortDate), Full),
    (C::Search(O::SortFrom), Full),
    (C::Search(O::SortTo), Full),
    (C::Search(O::SortSubject), Full),
    (C::Submit, Full),
    (C::SubmitCopy, Full),
];

/// SMTP (RFC 5321): sending only.
const SMTP: &[Row] = &[
    (C::Submit, Full),
    (
        C::SubmitCopy,
        Partial("himalaya files the copy into the storage backend after the send, two operations"),
    ),
];

/// A unified leaf command and the capabilities it reaches, for the help
/// and the MCP tool list.
pub struct Leaf {
    /// The command group, `message`.
    pub group: &'static str,
    /// The leaf under it, `read`.
    pub leaf: &'static str,
    /// What it reaches, any one of which served making it usable; none
    /// for a leaf needing no transport.
    pub capabilities: &'static [Capability],
    /// What an argument reaches beyond them, by its long name (`send`):
    /// the argument is usable only where its capability is served.
    pub optional: &'static [(&'static str, Capability)],
}

impl Leaf {
    /// Whether the account can run the leaf at all.
    pub fn is_served(&self, resolution: &Resolution) -> bool {
        self.capabilities.is_empty()
            || self
                .capabilities
                .iter()
                .any(|capability| resolution.support(*capability).is_served())
    }

    /// The arguments whose capability the account does not serve.
    pub fn unserved_args(&self, resolution: &Resolution) -> Vec<&'static str> {
        self.optional
            .iter()
            .filter(|(_, capability)| !resolution.support(*capability).is_served())
            .map(|(name, _)| *name)
            .collect()
    }
}

/// What the composers reach beyond what they read: a send, a save.
const COMPOSED: &[(&str, Capability)] = &[
    ("send", C::Submit),
    ("no-save", C::Submit),
    ("save", C::MessageAdd),
];

/// The flag capabilities, which `flag add`, `set` and `remove` reach.
const FLAGS: &[Capability] = &[
    C::FlagsSeen,
    C::FlagsAnswered,
    C::FlagsFlagged,
    C::FlagsDraft,
    C::FlagsImportant,
    C::FlagsJunk,
];

/// `envelope search` and the options of its query.
const SEARCH: &[Capability] = &[
    C::EnvelopeSearch,
    C::Search(O::Date),
    C::Search(O::After),
    C::Search(O::Before),
    C::Search(O::From),
    C::Search(O::To),
    C::Search(O::Cc),
    C::Search(O::Bcc),
    C::Search(O::Address),
    C::Search(O::Subject),
    C::Search(O::Body),
    C::Search(O::Flag),
    C::Search(O::Attachment),
    C::Search(O::SortDate),
    C::Search(O::SortFrom),
    C::Search(O::SortTo),
    C::Search(O::SortSubject),
];

/// Every unified leaf reaching a transport.
pub const LEAVES: &[Leaf] = &[
    Leaf::new("mailbox", "list", &[C::MailboxList]),
    Leaf::new("mailbox", "create", &[C::MailboxCreate]),
    Leaf::new("mailbox", "rename", &[C::MailboxRename]),
    Leaf::new("mailbox", "delete", &[C::MailboxDelete]),
    Leaf::new("envelope", "list", &[C::EnvelopeList]),
    Leaf::new("envelope", "search", SEARCH),
    Leaf::new("flag", "add", FLAGS),
    Leaf::new("flag", "set", FLAGS),
    Leaf::new("flag", "remove", FLAGS),
    Leaf::new("message", "add", &[C::MessageAdd]),
    Leaf::new("message", "copy", &[C::MessageCopy]),
    Leaf::new("message", "move", &[C::MessageMove]),
    Leaf::new(
        "message",
        "delete",
        &[C::MessageTrash, C::MessageMove, C::MessageRemove],
    ),
    Leaf::new("message", "read", &[C::MessageRead]),
    Leaf::new("message", "send", &[C::Submit, C::SubmitCopy]),
    Leaf::new("message", "compose", &[]).with(COMPOSED),
    Leaf::new("message", "reply", &[C::MessageRead]).with(COMPOSED),
    Leaf::new("message", "forward", &[C::MessageRead]).with(COMPOSED),
    Leaf::new("attachment", "list", &[C::AttachmentList]),
    Leaf::new("attachment", "download", &[C::AttachmentList]),
    Leaf::new("vacation", "get", &[C::Vacation]),
    Leaf::new("vacation", "set", &[C::Vacation]),
];

impl Leaf {
    const fn new(
        group: &'static str,
        leaf: &'static str,
        capabilities: &'static [Capability],
    ) -> Self {
        Self {
            group,
            leaf,
            capabilities,
            optional: &[],
        }
    }

    const fn with(self, optional: &'static [(&'static str, Capability)]) -> Self {
        Self { optional, ..self }
    }
}

/// Every capability a unified leaf reaches, once each, in the order the
/// leaves list them.
pub fn all() -> Vec<Capability> {
    let mut all = Vec::new();

    let optional = LEAVES
        .iter()
        .flat_map(|leaf| leaf.optional.iter().map(|(_, capability)| capability));

    for capability in LEAVES
        .iter()
        .flat_map(|leaf| leaf.capabilities)
        .chain(optional)
    {
        if !all.contains(capability) {
            all.push(*capability);
        }
    }

    all
}

/// The leaf of `group leaf`, `None` for one reaching no transport.
pub fn leaf(group: &str, leaf: &str) -> Option<&'static Leaf> {
    LEAVES
        .iter()
        .find(|known| known.group == group && known.leaf == leaf)
}

/// The `--help` footer of `group leaf`: for each capability it reaches,
/// the backends declaring it by level, then what each detail says. The
/// capabilities an argument reaches follow, the argument named.
///
/// Generated from the tables, so it never drifts from what the resolver
/// enforces. Empty for a leaf reaching no transport.
pub fn help(group: &str, leaf: &str) -> String {
    let Some(leaf) = self::leaf(group, leaf) else {
        return String::new();
    };

    let mut help = String::from("Backends:");

    // NOTE: each capability once, with the arguments reaching it.
    let mut reached: Vec<(&Capability, Vec<&str>)> = leaf
        .capabilities
        .iter()
        .map(|capability| (capability, Vec::new()))
        .collect();

    for (name, capability) in leaf.optional {
        match reached.iter_mut().find(|(known, _)| *known == capability) {
            Some((_, names)) => names.push(name),
            None => reached.push((capability, vec![name])),
        }
    }

    for (capability, args) in reached {
        let rows: Vec<(Backend, Support)> = Backend::ALL
            .iter()
            .filter_map(|backend| {
                declared(*backend)
                    .iter()
                    .find(|(declared, _)| declared == capability)
                    .map(|(_, support)| (*backend, *support))
            })
            .collect();

        let mut levels: Vec<(&str, Vec<String>)> = Vec::new();
        let mut details: Vec<(&str, Vec<String>)> = Vec::new();

        for (backend, support) in rows {
            group_by(&mut levels, support.level(), backend.to_string());
            if let Some(detail) = support.detail() {
                group_by(&mut details, detail, backend.to_string());
            }
        }

        let levels: Vec<String> = levels
            .into_iter()
            .map(|(level, backends)| format!("{level} on {}", backends.join(", ")))
            .collect();

        let with = match args.as_slice() {
            [] => String::new(),
            names => format!(" (with --{})", names.join(", --")),
        };
        help.push_str(&format!("\n  {capability}{with}: {}", levels.join("; ")));

        for (detail, backends) in details {
            help.push_str(&format!("\n    {}: {detail}", backends.join(", ")));
        }
    }

    help
}

/// Pushes `item` under `key`, opening the group on first sight so the
/// groups keep the order they were met in.
fn group_by<'a>(groups: &mut Vec<(&'a str, Vec<String>)>, key: &'a str, item: String) {
    match groups.iter_mut().find(|(known, _)| *known == key) {
        Some((_, items)) => items.push(item),
        None => groups.push((key, vec![item])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The footer names the capability an argument reaches, after those
    /// the command reaches whatever it is given.
    #[test]
    fn the_footer_names_what_an_argument_reaches() {
        let help = help("message", "reply");
        let read = help.find("message.read:").unwrap();
        let send = help.find("submit (with --send, --no-save):").unwrap();
        assert!(read < send, "{help}");
        assert!(help.contains("message.add (with --save):"), "{help}");
    }

    #[test]
    fn every_storage_backend_declares_the_verbs_exchanging_ids() {
        for backend in STORAGE_ORDER {
            for capability in [C::MailboxList, C::EnvelopeList, C::MessageRead] {
                assert_eq!(
                    support(*backend, capability),
                    Full,
                    "{backend} {capability}"
                );
            }
        }
    }

    #[test]
    fn a_backend_declares_a_capability_once() {
        for backend in Backend::ALL {
            let rows = declared(*backend);
            for (i, (capability, _)) in rows.iter().enumerate() {
                let again = rows[i + 1..].iter().any(|(other, _)| other == capability);
                assert!(!again, "{backend} declares {capability} twice");
            }
        }
    }

    #[test]
    fn a_name_follows_pimdirs_grammar_without_the_domain_word() {
        assert_eq!(C::MessageMove.name(), "message.move");
        assert_eq!(C::FlagsAnswered.name(), "flags.answered");
        assert_eq!(C::Search(O::SortDate).name(), "envelope.search.sort.date");
    }
}

#[cfg(all(
    test,
    feature = "imap",
    feature = "jmap",
    feature = "maildir",
    feature = "smtp"
))]
mod resolution_tests {
    use super::*;

    /// An account configuring `blocks`, its paths and servers never
    /// reached.
    fn account(blocks: &str) -> AccountConfig {
        toml::from_str(blocks).unwrap()
    }

    const MAILDIR_BLOCK: &str = "maildir.root = \"/nowhere\"\n";
    const IMAP_BLOCK: &str = "imap.server = \"imap://127.0.0.1:1\"\n";
    const JMAP_BLOCK: &str =
        "jmap.server = \"https://127.0.0.1:1\"\njmap.auth.bearer.token.command = \"false\"\n";
    const SMTP_BLOCK: &str = "smtp.server = \"127.0.0.1:1\"\n";

    #[test]
    fn the_ids_of_two_storage_backends_never_mix() {
        let config = account(&format!("{IMAP_BLOCK}{MAILDIR_BLOCK}"));
        let resolution = Resolution::of("a", &config, Backend::Auto);

        // NOTE: local before network, and every verb exchanging ids on
        // that one backend.
        for capability in [
            C::MailboxList,
            C::EnvelopeList,
            C::MessageRead,
            C::FlagsSeen,
        ] {
            assert_eq!(resolution.transport(capability), Some(Backend::Maildir));
        }

        let pinned = Resolution::of("a", &config, Backend::Imap);
        assert_eq!(pinned.transport(C::MessageRead), Some(Backend::Imap));
        assert_eq!(pinned.transport(C::EnvelopeList), Some(Backend::Imap));
    }

    #[test]
    fn the_auto_reply_resolves_to_the_backend_keeping_it() {
        // NOTE: a Maildir synced from JMAP stores the mail, JMAP keeps the
        // auto-reply.
        let config = account(&format!("{MAILDIR_BLOCK}{JMAP_BLOCK}"));
        let resolution = Resolution::of("a", &config, Backend::Auto);
        assert_eq!(resolution.storage, Some(Backend::Maildir));
        assert_eq!(resolution.transport(C::Vacation), Some(Backend::Jmap));

        // NOTE: no backend keeps one, so the storage backend refuses it,
        // naming what does.
        let imap = Resolution::of("a", &account(IMAP_BLOCK), Backend::Auto);
        assert_eq!(imap.transport(C::Vacation), Some(Backend::Imap));
        let refused = imap.require(&[C::Vacation]).unwrap_err().to_string();
        assert!(refused.contains("himalaya sieve"), "{refused}");
    }

    #[test]
    fn a_maildir_keyword_flag_needs_the_dovecot_keywords_file() {
        use pimalaya_cli::error::code_of;

        use crate::error::ErrorCode;

        let config = account(&format!(
            "{MAILDIR_BLOCK}maildir.keywords.header = \"x-label\"\n"
        ));
        let resolution = Resolution::of("a", &config, Backend::Auto);
        let err = resolution.require(&[C::FlagsImportant]).unwrap_err();
        assert_eq!(
            code_of::<ErrorCode>(&err),
            Some(ErrorCode::CapabilityMissing)
        );
        assert!(
            err.to_string().contains("maildir.keywords.dovecot"),
            "{err}"
        );

        let config = account(&format!("{MAILDIR_BLOCK}maildir.keywords.dovecot = true\n"));
        let resolution = Resolution::of("a", &config, Backend::Auto);
        assert!(
            resolution
                .require(&[C::FlagsImportant, C::FlagsJunk])
                .is_ok()
        );
    }

    #[test]
    fn sending_resolves_on_its_own() {
        let imap = account(&format!("{IMAP_BLOCK}{SMTP_BLOCK}"));
        let resolution = Resolution::of("a", &imap, Backend::Auto);
        assert_eq!(resolution.storage, Some(Backend::Imap));
        assert_eq!(resolution.sender, Some(Backend::Smtp));

        let jmap = account(&format!("{JMAP_BLOCK}{SMTP_BLOCK}"));
        let resolution = Resolution::of("a", &jmap, Backend::Auto);
        assert_eq!(resolution.sender, Some(Backend::Jmap));

        let smtp_only = Resolution::of("a", &jmap, Backend::Smtp);
        assert_eq!(smtp_only.storage, None);
        assert_eq!(smtp_only.sender, Some(Backend::Smtp));
    }
}

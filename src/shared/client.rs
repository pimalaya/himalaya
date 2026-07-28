//! Cross-protocol [`EmailClient`] for the shared subcommands
//! (`mailboxes`, `envelopes`, `flags`, `messages`, `attachments`).
//!
//! Mirrors cardamum's structure: a single storage backend (the first
//! configured one the [`Backend`] flag allows, in local-before-network
//! priority) held in a [`BackendClient`] enum, plus an optional SMTP
//! transport for accounts whose storage backend cannot send (IMAP,
//! Maildir, m2dir). Each shared method matches the active backend and
//! calls its adapter (the per-protocol `<proto>/backend.rs`), which
//! takes and returns the CLI's shared [`crate::email`] types.
//!
//! The active [`Account`] is threaded as a sibling argument through
//! every `execute` chain rather than being bundled into the client.

#[cfg(feature = "smtp")]
use std::mem;

use anyhow::{Result, anyhow, bail};

#[cfg(feature = "gmail")]
use crate::gmail::client::GmailClient;
#[cfg(feature = "imap")]
use crate::imap::client::ImapClient;
#[cfg(feature = "jmap")]
use crate::jmap::client::JmapClient;
#[cfg(feature = "m2dir")]
use crate::m2dir::client::M2dirClient;
#[cfg(feature = "maildir")]
use crate::maildir::client::MaildirClient;
#[cfg(feature = "msgraph")]
use crate::msgraph::client::MsgraphClient;
// `Envelope`/`Mailbox`/`FlagOp`/`SearchEmailsQuery` are only used by the
// mailbox-dispatch methods, so they carry the same `backend` gate;
// `Flag` is also used by the always-compiled `add_message`.
#[cfg(backend)]
use crate::email::{
    envelope::Envelope, flag::FlagOp, mailbox::Mailbox, search::query::SearchEmailsQuery,
};
use crate::{
    account::context::Account,
    backend::Backend,
    config::{AccountConfig, Config},
    email::flag::Flag,
};
#[cfg(feature = "smtp")]
use crate::{config::SmtpConfig, smtp::client::SmtpClient};

/// Cross-protocol email client backing the shared subcommands.
pub struct EmailClient {
    storage: Option<BackendClient>,
    #[cfg(feature = "smtp")]
    smtp: SmtpTransport,
}

/// The SMTP transport slot, connected lazily on the first send so that
/// read-only commands never open an SMTP connection.
#[cfg(feature = "smtp")]
enum SmtpTransport {
    /// No SMTP configured, or excluded by `--backend`.
    Absent,
    /// Configured but not yet connected.
    Pending(Box<SmtpConfig>),
    /// Connected.
    Ready(SmtpClient),
}

/// The active storage backend: exactly one of the compiled-in
/// per-protocol clients.
enum BackendClient {
    #[cfg(feature = "imap")]
    Imap(Box<ImapClient>),
    #[cfg(feature = "jmap")]
    Jmap(Box<JmapClient>),
    #[cfg(feature = "gmail")]
    Gmail(Box<GmailClient>),
    #[cfg(feature = "msgraph")]
    Msgraph(Box<MsgraphClient>),
    #[cfg(feature = "maildir")]
    Maildir(Box<MaildirClient>),
    #[cfg(feature = "m2dir")]
    M2dir(Box<M2dirClient>),
}

impl EmailClient {
    /// Opens the connections for the active account: the first
    /// configured storage backend the `backend` flag allows, plus an
    /// SMTP transport when one is configured. Bails when nothing usable
    /// is configured.
    pub fn new(
        config: Config,
        #[allow(unused_mut)] mut account_config: AccountConfig,
        backend: Backend,
    ) -> Result<(Account, Self)> {
        let storage = select_storage(&mut account_config, backend)?;

        // NOTE: kept unconnected here; a read-only command must not open
        // an SMTP connection (it also lets a single-session proxy such as
        // sirup serve the storage backend without a second client).
        #[cfg(feature = "smtp")]
        let smtp = match (backend.allows_smtp(), account_config.smtp.take()) {
            (true, Some(config)) => SmtpTransport::Pending(Box::new(config)),
            _ => SmtpTransport::Absent,
        };

        #[cfg(feature = "smtp")]
        let has_transport = storage.is_some() || !matches!(smtp, SmtpTransport::Absent);
        #[cfg(not(feature = "smtp"))]
        let has_transport = storage.is_some();
        if !has_transport {
            bail!("No backend matching `{backend}` is configured for this account");
        }

        let account = Account::from(config).merge(Account::from(account_config));

        Ok((
            account,
            Self {
                storage,
                #[cfg(feature = "smtp")]
                smtp,
            },
        ))
    }

    /// Lists every mailbox available to the active account.
    #[cfg(backend)]
    pub fn list_mailboxes(&mut self, with_counts: bool) -> Result<Vec<Mailbox>> {
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.list_mailboxes(with_counts),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.list_mailboxes(with_counts),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.list_mailboxes(with_counts),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.list_mailboxes(with_counts),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.list_mailboxes(with_counts),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.list_mailboxes(with_counts),
        }
    }

    /// Lists envelopes from `mailbox`.
    #[cfg(backend)]
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => {
                client.list_envelopes(mailbox, page, page_size, with_attachment)
            }
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => {
                client.list_envelopes(mailbox, page, page_size, with_attachment)
            }
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => {
                client.list_envelopes(mailbox, page, page_size, with_attachment)
            }
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => {
                client.list_envelopes(mailbox, page, page_size, with_attachment)
            }
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => {
                client.list_envelopes(mailbox, page, page_size, with_attachment)
            }
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => {
                client.list_envelopes(mailbox, page, page_size, with_attachment)
            }
        }
    }

    /// Searches envelopes in `mailbox` against the shared query DSL.
    /// Gmail and Microsoft Graph do not implement the shared search.
    #[cfg(backend)]
    pub fn search_envelopes(
        &mut self,
        mailbox: &str,
        query: Option<&SearchEmailsQuery>,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => {
                client.search_envelopes(mailbox, query, page, page_size, with_attachment)
            }
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => {
                client.search_envelopes(mailbox, query, page, page_size, with_attachment)
            }
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => {
                client.search_envelopes(mailbox, query, page, page_size, with_attachment)
            }
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => {
                client.search_envelopes(mailbox, query, page, page_size, with_attachment)
            }
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(_) => bail!("Gmail does not support the shared envelope search"),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(_) => {
                bail!("Microsoft Graph does not support the shared envelope search")
            }
        }
    }

    /// Adds, sets, or removes `flags` on a message id set in `mailbox`.
    #[cfg(backend)]
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.store_flags(mailbox, ids, flags, op),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.store_flags(mailbox, ids, flags, op),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.store_flags(mailbox, ids, flags, op),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.store_flags(mailbox, ids, flags, op),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.store_flags(mailbox, ids, flags, op),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.store_flags(mailbox, ids, flags, op),
        }
    }

    /// Fetches one message's raw RFC 5322 bytes. When `seen` is set, the
    /// message is also marked as seen: IMAP folds this into the fetch
    /// (`BODY[]`), the other backends issue a separate flag update.
    #[cfg(backend)]
    pub fn get_message(&mut self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.get_message(mailbox, id, seen),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.get_message(mailbox, id, seen),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.get_message(mailbox, id, seen),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.get_message(mailbox, id, seen),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.get_message(mailbox, id, seen),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.get_message(mailbox, id, seen),
        }
    }

    /// Adds `raw` to `mailbox` with `flags`. Gmail and Microsoft Graph
    /// do not implement adding messages.
    pub fn add_message(&mut self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.add_message(mailbox, flags, raw),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.add_message(mailbox, flags, raw),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.add_message(mailbox, flags, raw),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.add_message(mailbox, flags, raw),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(_) => bail!("Gmail does not support adding messages"),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(_) => bail!("Microsoft Graph does not support adding messages"),
            // No storage backend compiled in (send-only build): `save`
            // has nowhere to land. `storage_mut()` bails first, so this
            // arm only keeps the match exhaustive over the empty enum.
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Copies a message id set from `from` to `to`, returning the number
    /// actually affected (see the per-backend adapters).
    #[cfg(backend)]
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let from = self.resolve_mailbox_id(from)?;
        let to = self.resolve_mailbox_id(to)?;
        let (from, to) = (from.as_str(), to.as_str());
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.copy_messages(from, to, ids),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.copy_messages(from, to, ids),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.copy_messages(from, to, ids),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.copy_messages(from, to, ids),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.copy_messages(from, to, ids),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.copy_messages(from, to, ids),
        }
    }

    /// Moves a message id set from `from` to `to`, returning the number
    /// actually affected (see the per-backend adapters).
    #[cfg(backend)]
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let from = self.resolve_mailbox_id(from)?;
        let to = self.resolve_mailbox_id(to)?;
        let (from, to) = (from.as_str(), to.as_str());
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.move_messages(from, to, ids),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.move_messages(from, to, ids),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.move_messages(from, to, ids),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.move_messages(from, to, ids),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.move_messages(from, to, ids),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.move_messages(from, to, ids),
        }
    }

    /// The backend's own trash mailbox id, when it can resolve one
    /// without configuration (JMAP `role=trash`, Gmail `TRASH` label,
    /// Graph `deleteditems`). IMAP, Maildir and m2dir return `None`, so
    /// the caller falls back to the `mailbox.alias.trash` config entry.
    /// The trash-first delete policy lives in `shared::message::delete`.
    #[cfg(backend)]
    pub fn native_trash(&mut self) -> Result<Option<String>> {
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(_) => Ok(None),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.native_trash(),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(_) => Ok(Some(String::from("TRASH"))),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(_) => Ok(Some(String::from("deleteditems"))),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(_) => Ok(None),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(_) => Ok(None),
        }
    }

    /// Permanently deletes `ids` from `mailbox` (assumed to be the trash).
    /// Returns `true` when they were physically removed, `false` when
    /// only flagged (IMAP without UIDPLUS). The move-vs-delete decision
    /// is made by the caller (`shared::message::delete`).
    #[cfg(backend)]
    pub fn delete_messages(&mut self, mailbox: &str, ids: &[&str]) -> Result<bool> {
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.delete_messages(mailbox, ids),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.delete_messages(ids).map(|()| true),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.delete_messages(ids).map(|()| true),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.delete_messages(ids).map(|()| true),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.delete_messages(mailbox, ids).map(|()| true),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.delete_messages(mailbox, ids).map(|()| true),
        }
    }

    /// Sends `raw`: through the storage backend when it can send itself
    /// (JMAP, Gmail, Graph), otherwise through the SMTP transport.
    pub fn send_message(&mut self, raw: Vec<u8>) -> Result<()> {
        match &mut self.storage {
            #[cfg(feature = "jmap")]
            Some(BackendClient::Jmap(client)) => return client.send_message(raw),
            #[cfg(feature = "gmail")]
            Some(BackendClient::Gmail(client)) => return client.send_message(raw),
            #[cfg(feature = "msgraph")]
            Some(BackendClient::Msgraph(client)) => return client.send_message(raw),
            _ => {}
        }

        #[cfg(feature = "smtp")]
        if let Some(smtp) = self.smtp_client_mut()? {
            return smtp.send_message(raw);
        }

        bail!("No send-capable backend (JMAP/Gmail/Graph) or SMTP is configured for this account")
    }

    /// Connects the SMTP transport on first use, returning `None` when no
    /// SMTP is configured. Only the send path calls this, so read-only
    /// commands open no SMTP connection.
    #[cfg(feature = "smtp")]
    fn smtp_client_mut(&mut self) -> Result<Option<&mut SmtpClient>> {
        if let SmtpTransport::Pending(_) = &self.smtp {
            let SmtpTransport::Pending(config) =
                mem::replace(&mut self.smtp, SmtpTransport::Absent)
            else {
                unreachable!()
            };
            self.smtp = SmtpTransport::Ready(SmtpClient::new(*config)?);
        }

        Ok(match &mut self.smtp {
            SmtpTransport::Ready(client) => Some(client),
            _ => None,
        })
    }

    /// Maps the shared layer's human mailbox name onto the
    /// backend-native id the operation methods expect. Identity for
    /// every backend whose name already is its id (IMAP, Maildir,
    /// m2dir); JMAP resolves the opaque mailbox id via a cached
    /// `Mailbox/get`, Gmail the opaque label id via a cached
    /// `labels.list`, and Microsoft Graph the opaque folder id via a
    /// cached `mailFolders` listing (Graph well-known names still pass
    /// through). Applied by the mailbox-addressing methods above before
    /// they dispatch, so each per-protocol adapter only ever receives
    /// ids. Idempotent: an already-resolved id passes through unchanged,
    /// so `shared::message::delete` uses it to compare a mailbox against
    /// the trash.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        match self.storage_mut()? {
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.resolve_mailbox_id(mailbox),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.resolve_mailbox_id(mailbox),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.resolve_mailbox_id(mailbox),
            _ => Ok(mailbox.to_string()),
        }
    }

    fn storage_mut(&mut self) -> Result<&mut BackendClient> {
        self.storage
            .as_mut()
            .ok_or_else(|| anyhow!("No storage backend is configured for this account"))
    }
}

/// Picks the storage backend for the account: the first configured one
/// the `backend` flag allows, local before network to match the retired
/// io-email dispatcher's read priority.
#[cfg_attr(
    not(any(
        feature = "maildir",
        feature = "m2dir",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph",
        feature = "imap"
    )),
    allow(unused_variables)
)]
fn select_storage(
    account_config: &mut AccountConfig,
    backend: Backend,
) -> Result<Option<BackendClient>> {
    #[cfg(feature = "maildir")]
    if backend.allows_maildir()
        && let Some(config) = account_config.maildir.take()
    {
        return Ok(Some(BackendClient::Maildir(Box::new(MaildirClient::new(
            config,
        )))));
    }

    #[cfg(feature = "m2dir")]
    if backend.allows_m2dir()
        && let Some(config) = account_config.m2dir.take()
    {
        return Ok(Some(BackendClient::M2dir(Box::new(M2dirClient::new(
            config,
        )))));
    }

    #[cfg(feature = "jmap")]
    if backend.allows_jmap()
        && let Some(config) = account_config.jmap.take()
    {
        return Ok(Some(BackendClient::Jmap(Box::new(JmapClient::new(
            config,
        )?))));
    }

    #[cfg(feature = "gmail")]
    if backend.allows_gmail()
        && let Some(config) = account_config.gmail.take()
    {
        return Ok(Some(BackendClient::Gmail(Box::new(GmailClient::new(
            config,
        )?))));
    }

    #[cfg(feature = "msgraph")]
    if backend.allows_msgraph()
        && let Some(config) = account_config.msgraph.take()
    {
        return Ok(Some(BackendClient::Msgraph(Box::new(MsgraphClient::new(
            config,
        )?))));
    }

    #[cfg(feature = "imap")]
    if backend.allows_imap()
        && let Some(config) = account_config.imap.take()
    {
        return Ok(Some(BackendClient::Imap(Box::new(ImapClient::new(
            config,
        )?))));
    }

    Ok(None)
}

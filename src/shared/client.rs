//! # Email client
//!
//! The cross-protocol client every shared subcommand runs over: one
//! storage backend, plus an SMTP transport for the backends that cannot
//! send.
//!
//! The storage backend is the first configured one `--backend` allows,
//! local before network. Each method matches on it and calls that
//! protocol's adapter, which takes and returns the CLI's own
//! [`crate::email`] types.
//!
//! The active [`Account`] is threaded as a sibling argument through every
//! `execute` chain rather than bundled into the client.

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
#[cfg(feature = "pimdir")]
use crate::pimdir::client::PimdirClient;
// NOTE: the mailbox types carry the same `backend` gate as the methods
// dispatching on them, where `Flag` also serves the ungated send path.
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

/// The client every shared subcommand runs over.
pub struct EmailClient {
    storage: Option<BackendClient>,
    #[cfg(feature = "smtp")]
    smtp: SmtpTransport,
}

/// The SMTP transport, connected on the first send so a read-only command
/// opens no SMTP connection.
#[cfg(feature = "smtp")]
enum SmtpTransport {
    /// No SMTP configured, or `--backend` excluded it.
    Absent,
    /// Configured but not connected yet.
    Pending(Box<SmtpConfig>),
    /// Connected.
    Ready(SmtpClient),
}

/// The active storage backend, one of the compiled-in per-protocol
/// clients.
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
    #[cfg(feature = "pimdir")]
    Pimdir(Box<PimdirClient>),
}

impl EmailClient {
    /// Opens the connections of the active account, bailing when nothing
    /// usable is configured.
    pub fn new(
        config: Config,
        #[allow(unused_mut)] mut account_config: AccountConfig,
        backend: Backend,
    ) -> Result<(Account, Self)> {
        let storage = select_storage(&mut account_config, backend)?;

        // NOTE: left unconnected, so a read-only command opens no SMTP
        // connection and a single-session proxy serves storage alone.
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
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.list_mailboxes(with_counts),
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
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => {
                client.list_envelopes(mailbox, page, page_size, with_attachment)
            }
        }
    }

    /// How many messages the mailbox has staged for creation and not
    /// pushed yet.
    ///
    /// Zero for every backend whose writes reach the server as they are
    /// made. A pimdir store is a replica a sync engine owns, so a saved
    /// message waits in its queue with no id and therefore no envelope,
    /// and the count is what keeps it from reading as lost.
    #[cfg(backend)]
    pub fn queued_messages(&mut self, mailbox: &str) -> Result<usize> {
        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();
        match self.storage_mut()? {
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.queued_messages(mailbox),
            #[allow(unreachable_patterns)]
            _ => Ok(0),
        }
    }

    /// Searches a mailbox with the shared query, which Gmail and
    /// Microsoft Graph do not implement.
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
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => {
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

    /// Adds, sets or removes flags on a set of messages.
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
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.store_flags(mailbox, ids, flags, op),
        }
    }

    /// Fetches one message's raw RFC 5322 bytes, marking it seen when
    /// asked.
    ///
    /// IMAP folds the flag into the fetch itself, where the other
    /// backends issue a separate update.
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
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.get_message(mailbox, id, seen),
        }
    }

    /// Adds a raw message to a mailbox with the given flags, which Gmail
    /// and Microsoft Graph do not implement.
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
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.add_message(mailbox, flags, raw),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(_) => bail!("Gmail does not support adding messages"),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(_) => bail!("Microsoft Graph does not support adding messages"),
            // NOTE: `storage_mut` bails first in a send-only build, so
            // this arm only keeps the match exhaustive over an empty enum.
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Copies messages between two mailboxes, returning how many landed.
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
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.copy_messages(from, to, ids),
        }
    }

    /// Moves messages between two mailboxes, returning how many landed.
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
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.move_messages(from, to, ids),
        }
    }

    /// The trash mailbox the backend names on its own, `None` when it
    /// names none.
    ///
    /// JMAP, Gmail and Graph each carry a well-known trash, where IMAP,
    /// Maildir and m2dir leave the caller to fall back on the
    /// `mailbox.alias.trash` entry.
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
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(_) => Ok(None),
        }
    }

    /// Permanently deletes messages from the trash.
    ///
    /// Returns whether they were really removed, an IMAP server without
    /// UIDPLUS only flagging them `\Deleted`.
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
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.delete_messages(mailbox, ids).map(|()| true),
        }
    }

    /// Sends a raw message through the storage backend when it can send,
    /// and through the SMTP transport otherwise.
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

    /// Connects the SMTP transport on first use, `None` when none is
    /// configured.
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

    /// Maps a human mailbox name onto the backend-native id every
    /// operation method expects.
    ///
    /// Identity where the name already is the id, and a cached listing
    /// where the backend mints an opaque one. Every dispatching method
    /// runs it first, so an adapter only ever receives ids.
    ///
    /// Idempotent, an already-resolved id passing through unchanged,
    /// which is what lets a caller compare a mailbox against the trash.
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

/// Picks the account's storage backend: the first configured one
/// `--backend` allows, local before network.
#[cfg_attr(
    not(any(
        feature = "maildir",
        feature = "m2dir",
        feature = "pimdir",
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

    #[cfg(feature = "pimdir")]
    if backend.allows_pimdir()
        && let Some(config) = account_config.pimdir.take()
    {
        return Ok(Some(BackendClient::Pimdir(Box::new(PimdirClient::new(
            config,
        )?))));
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

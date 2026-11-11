//! # Email client
//!
//! The cross-protocol client every shared subcommand runs over: one
//! storage backend, plus an SMTP transport for the backends that cannot
//! send.
//!
//! The storage backend is the one [`Resolution`] picks: the first
//! configured one `--backend` allows, local before network. Each method
//! matches on it and calls that protocol's adapter, which takes and returns
//! the CLI's own [`crate::email`] types.
//!
//! The active [`Account`] is threaded as a sibling argument through every
//! `execute` chain rather than bundled into the client.

// NOTE: a send-only build compiles every storage method with no arm to
// use its parameters, the resolver refusing the verb before any runs.
#![cfg_attr(not(backend), allow(unused_variables))]

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
#[cfg(feature = "mbox")]
use crate::mbox::client::MboxClient;
#[cfg(feature = "msgraph")]
use crate::msgraph::client::MsgraphClient;
#[cfg(feature = "pimdir")]
use crate::pimdir::client::PimdirClient;
use crate::{
    account::context::Account,
    backend::Backend,
    capability::{Capability, Resolution},
    config::{AccountConfig, Config},
    email::{
        envelope::Envelope,
        flag::{Flag, FlagOp},
        mailbox::{Mailbox, MailboxRole},
        search::query::SearchEmailsQuery,
    },
};
#[cfg(feature = "smtp")]
use crate::{config::SmtpConfig, smtp::client::SmtpClient};

/// The client every shared subcommand runs over.
pub struct EmailClient {
    /// The transports the account's verbs resolve to.
    resolution: Resolution,
    storage: Option<BackendClient>,
    /// The SMTP transport, `None` when none is configured or `--backend`
    /// excludes it.
    ///
    /// Connected for each send and closed after it, so a read-only command
    /// opens no SMTP connection, and a session never sends over one a
    /// server dropped while idle.
    #[cfg(feature = "smtp")]
    smtp: Option<Box<SmtpConfig>>,
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
    #[cfg(feature = "mbox")]
    Mbox(Box<MboxClient>),
    #[cfg(feature = "pimdir")]
    Pimdir(Box<PimdirClient>),
}

impl EmailClient {
    /// Opens the connections of the active account, bailing when nothing
    /// usable is configured.
    pub fn new(
        config: Config,
        name: &str,
        #[allow(unused_mut)] mut account_config: AccountConfig,
        backend: Backend,
    ) -> Result<(Account, Self)> {
        let resolution = Resolution::of(name, &account_config, backend);
        let storage = open_storage(&mut account_config, resolution.storage)?;

        // NOTE: left unconnected, so a read-only command opens no SMTP
        // connection and a single-session proxy serves storage alone.
        #[cfg(feature = "smtp")]
        let smtp = match (backend.allows_smtp(), account_config.smtp.take()) {
            (true, Some(config)) => Some(Box::new(config)),
            _ => None,
        };

        #[cfg(feature = "smtp")]
        let has_transport = storage.is_some() || smtp.is_some();
        #[cfg(not(feature = "smtp"))]
        let has_transport = storage.is_some();
        if !has_transport {
            // NOTE: ManageSieve holds no mailbox, so no shared command runs on it.
            let supported: Vec<Backend> = Backend::COMPILED
                .iter()
                .copied()
                .filter(|b| *b != Backend::Sieve)
                .collect();

            return Err(account_config.no_backend_error(name, backend, &supported));
        }

        let account = Account::from(config).merge(Account::from(account_config));

        Ok((
            account,
            Self {
                resolution,
                storage,
                #[cfg(feature = "smtp")]
                smtp,
            },
        ))
    }

    /// Lists every mailbox available to the active account.
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
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client.list_mailboxes(with_counts),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.list_mailboxes(with_counts),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Creates the mailbox `name` at the top, or under `parent`,
    /// returning its id, `None` when the backend queues the creation.
    pub fn create_mailbox(&mut self, name: &str, parent: Option<&str>) -> Result<Option<String>> {
        self.require(&[Capability::MailboxCreate])?;

        let parent = match parent {
            Some(parent) => Some(self.resolve_mailbox_id(parent)?),
            None => None,
        };
        let parent = parent.as_deref();

        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.create_mailbox(name, parent).map(Some),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.create_mailbox(name, parent).map(Some),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.create_mailbox(name, parent).map(Some),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.create_mailbox(name, parent).map(Some),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.create_mailbox(name, parent).map(Some),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.create_mailbox(name, parent).map(Some),
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client.create_mailbox(name, parent).map(Some),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.create_mailbox(name, parent).map(|_| None),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Renames the last level of `mailbox` to `name`, its parent kept,
    /// returning the id it has afterwards, `None` when the backend queues
    /// the rename.
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
        allow(unused_variables)
    )]
    pub fn rename_mailbox(&mut self, mailbox: &str, name: &str) -> Result<Option<String>> {
        self.require(&[Capability::MailboxRename])?;

        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.rename_mailbox(mailbox, name).map(Some),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.rename_mailbox(mailbox, name).map(Some),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.rename_mailbox(mailbox, name).map(Some),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.rename_mailbox(mailbox, name).map(Some),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.rename_mailbox(mailbox, name).map(Some),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(_) => bail!("m2dir has no rename yet"),
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client.rename_mailbox(mailbox, name).map(Some),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.rename_mailbox(mailbox, name).map(|_| None),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Deletes `mailbox` and the messages it holds, returning its id.
    #[cfg_attr(not(backend), allow(unreachable_code))]
    pub fn delete_mailbox(&mut self, mailbox: &str) -> Result<String> {
        self.require(&[Capability::MailboxDelete])?;

        let mailbox = self.resolve_mailbox_id(mailbox)?;
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.delete_mailbox(&mailbox)?,
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.delete_mailbox(&mailbox)?,
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.delete_mailbox(&mailbox)?,
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.delete_mailbox(&mailbox)?,
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.delete_mailbox(&mailbox)?,
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.delete_mailbox(&mailbox)?,
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client.delete_mailbox(&mailbox)?,
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => {
                client.delete_mailbox(&mailbox)?;
            }
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }

        Ok(mailbox)
    }

    /// Lists envelopes from `mailbox`.
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
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => {
                client.list_envelopes(mailbox, page, page_size, with_attachment)
            }
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => {
                client.list_envelopes(mailbox, page, page_size, with_attachment)
            }
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Searches a mailbox with the shared query.
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
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => {
                client.search_envelopes(mailbox, query, page, page_size, with_attachment)
            }
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => {
                client.search_envelopes(mailbox, query, page, page_size, with_attachment)
            }
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => {
                client.search_envelopes(mailbox, query, page, page_size, with_attachment)
            }
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => {
                client.search_envelopes(mailbox, query, page, page_size, with_attachment)
            }
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Adds, sets or removes flags on a set of messages, returning how
    /// many of them were found.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<usize> {
        let capabilities: Vec<Capability> = flags.iter().filter_map(Flag::capability).collect();
        self.require(&capabilities)?;

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
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client.store_flags(mailbox, ids, flags, op),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.store_flags(mailbox, ids, flags, op),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Fetches one message's raw RFC 5322 bytes, marking it seen when
    /// asked.
    ///
    /// IMAP folds the flag into the fetch itself, where the other
    /// backends issue a separate update.
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
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client.get_message(mailbox, id, seen),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.get_message(mailbox, id, seen),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Adds a raw message to a mailbox with the given flags, which Gmail
    /// and Microsoft Graph do not implement. Returns the new message id,
    /// absent when an IMAP or JMAP server does not report it.
    #[cfg_attr(
        not(any(
            feature = "imap",
            feature = "jmap",
            feature = "gmail",
            feature = "maildir",
            feature = "m2dir",
            feature = "mbox",
            feature = "pimdir"
        )),
        allow(unused_variables)
    )]
    pub fn add_message(
        &mut self,
        mailbox: &str,
        flags: &[Flag],
        raw: Vec<u8>,
    ) -> Result<Option<String>> {
        self.require(&[Capability::MessageAdd])?;

        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.add_message(mailbox, flags, raw),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.add_message(mailbox, flags, raw),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.add_message(mailbox, flags, raw).map(Some),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.add_message(mailbox, flags, raw).map(Some),
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client.add_message(mailbox, flags, raw).map(Some),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client
                .add_message(mailbox, flags, raw)
                .map(|staged| Some(staged.message_id)),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.add_message(mailbox, flags, raw).map(Some),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(_) => bail!("Microsoft Graph does not support adding messages"),
            // NOTE: `storage_mut` bails first in a send-only build, so
            // this arm only keeps the match exhaustive over an empty enum.
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Copies messages between two mailboxes, returning how many landed.
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
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client.copy_messages(from, to, ids),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.copy_messages(from, to, ids),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Moves messages between two mailboxes, returning how many landed.
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
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client.move_messages(from, to, ids),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.move_messages(from, to, ids),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// The id of the mailbox the backend marks with `role`, `None` when it
    /// marks none.
    ///
    /// JMAP, Gmail, Graph and pimdir carry roles, IMAP only the reserved
    /// `INBOX` until SPECIAL-USE is read, mbox the spool as `INBOX`, and
    /// Maildir and m2dir none, the caller falling back on the
    /// `mailbox.alias.<role>` entry.
    #[cfg_attr(
        not(any(
            feature = "imap",
            feature = "jmap",
            feature = "gmail",
            feature = "msgraph",
            feature = "mbox",
            feature = "pimdir"
        )),
        allow(unused_variables)
    )]
    pub fn role_mailbox_id(&mut self, role: &MailboxRole) -> Result<Option<String>> {
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(_) => {
                Ok((*role == MailboxRole::Inbox).then(|| String::from("INBOX")))
            }
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.role_mailbox_id(role),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.role_mailbox_id(role),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.role_mailbox_id(role),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(_) => Ok(None),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(_) => Ok(None),
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => Ok(client.role_mailbox_id(role)),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.role_mailbox_id(role),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Permanently deletes messages from the trash.
    ///
    /// Returns whether they were really removed, an IMAP server without
    /// UIDPLUS only flagging them `\Deleted`.
    #[cfg_attr(
        not(any(
            feature = "imap",
            feature = "maildir",
            feature = "m2dir",
            feature = "mbox",
            feature = "pimdir"
        )),
        allow(unused_variables)
    )]
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
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client.delete_messages(mailbox, ids).map(|()| true),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.delete_messages(mailbox, ids).map(|_| true),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Refuses the first of `capabilities` the account does not serve,
    /// before anything is sent.
    pub fn require(&self, capabilities: &[Capability]) -> Result<()> {
        self.resolution.require(capabilities)
    }

    /// Whether the storage backend queues its writes for a sync engine to
    /// apply later rather than performing them, so a confirmation says the
    /// write is queued rather than done.
    pub fn queues_writes(&self) -> bool {
        #[cfg(feature = "pimdir")]
        if let Some(BackendClient::Pimdir(_)) = &self.storage {
            return true;
        }
        false
    }

    /// Sends a raw message through the storage backend when it can send,
    /// and through the SMTP transport otherwise.
    ///
    /// `mailbox` is the sent mailbox, and `copy` asks for the message to
    /// be filed there. A backend may file that copy itself as part of the
    /// send and return `true`; most ignore both and return `false`, the
    /// copy being the caller's.
    #[cfg_attr(not(feature = "pimdir"), allow(unused_variables))]
    pub fn send_message(
        &mut self,
        mailbox: Option<&str>,
        raw: Vec<u8>,
        copy: bool,
    ) -> Result<bool> {
        match &mut self.storage {
            #[cfg(feature = "jmap")]
            Some(BackendClient::Jmap(client)) => return client.send_message(raw).map(|()| false),
            #[cfg(feature = "gmail")]
            Some(BackendClient::Gmail(client)) => return client.send_message(raw).map(|()| false),
            #[cfg(feature = "msgraph")]
            Some(BackendClient::Msgraph(client)) => {
                return client.send_message(raw).map(|()| false);
            }
            #[cfg(feature = "pimdir")]
            Some(BackendClient::Pimdir(client)) => {
                return client
                    .send_message(mailbox, raw, copy)
                    .map(|sent| sent.carried);
            }
            _ => {}
        }

        #[cfg(feature = "smtp")]
        if let Some(config) = &self.smtp {
            let mut smtp = SmtpClient::new(config.as_ref().clone())?;
            return smtp.send_message(raw).map(|()| false);
        }

        bail!(
            "No send-capable backend (JMAP/Gmail/Graph/pimdir) or SMTP is configured for this account"
        )
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
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.resolve_mailbox_id(mailbox),
            #[allow(unreachable_patterns)]
            _ => Ok(mailbox.to_string()),
        }
    }

    fn storage_mut(&mut self) -> Result<&mut BackendClient> {
        self.storage
            .as_mut()
            .ok_or_else(|| anyhow!("No storage backend is configured for this account"))
    }
}

/// Opens the storage backend the resolver picked, taking its block out
/// of the account.
#[cfg_attr(
    not(any(
        feature = "maildir",
        feature = "m2dir",
        feature = "mbox",
        feature = "pimdir",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph",
        feature = "imap"
    )),
    allow(unused_variables)
)]
fn open_storage(
    account_config: &mut AccountConfig,
    storage: Option<Backend>,
) -> Result<Option<BackendClient>> {
    let client = match storage {
        #[cfg(feature = "maildir")]
        Some(Backend::Maildir) => account_config
            .maildir
            .take()
            .map(|config| BackendClient::Maildir(Box::new(MaildirClient::new(config)))),
        #[cfg(feature = "m2dir")]
        Some(Backend::M2dir) => account_config
            .m2dir
            .take()
            .map(|config| BackendClient::M2dir(Box::new(M2dirClient::new(config)))),
        #[cfg(feature = "mbox")]
        Some(Backend::Mbox) => account_config
            .mbox
            .take()
            .map(|config| BackendClient::Mbox(Box::new(MboxClient::new(config)))),
        #[cfg(feature = "pimdir")]
        Some(Backend::Pimdir) => match account_config.pimdir.take() {
            Some(config) => Some(BackendClient::Pimdir(Box::new(PimdirClient::new(config)?))),
            None => None,
        },
        #[cfg(feature = "jmap")]
        Some(Backend::Jmap) => match account_config.jmap.take() {
            Some(config) => Some(BackendClient::Jmap(Box::new(JmapClient::new(config)?))),
            None => None,
        },
        #[cfg(feature = "gmail")]
        Some(Backend::Gmail) => match account_config.gmail.take() {
            Some(config) => Some(BackendClient::Gmail(Box::new(GmailClient::new(config)?))),
            None => None,
        },
        #[cfg(feature = "msgraph")]
        Some(Backend::Msgraph) => match account_config.msgraph.take() {
            Some(config) => Some(BackendClient::Msgraph(Box::new(MsgraphClient::new(
                config,
            )?))),
            None => None,
        },
        #[cfg(feature = "imap")]
        Some(Backend::Imap) => match account_config.imap.take() {
            Some(config) => Some(BackendClient::Imap(Box::new(ImapClient::new(config)?))),
            None => None,
        },
        _ => None,
    };

    Ok(client)
}

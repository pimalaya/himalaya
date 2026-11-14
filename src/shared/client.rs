//! # Email client
//!
//! The cross-protocol client every unified subcommand runs over: one
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

#[cfg(feature = "gmail")]
use anyhow::Context;
use anyhow::{Result, anyhow, bail};
use pimalaya_config::secret::SecretResolver;

#[cfg(feature = "gmail")]
use crate::gmail::client::GmailClient;
#[cfg(feature = "imap")]
use crate::imap::client::ImapClient;
#[cfg(feature = "jmap")]
use crate::jmap::client::JmapClient;
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
        mailbox::{self, Mailbox, MailboxRole},
        search::query::SearchEmailsQuery,
    },
};
#[cfg(feature = "smtp")]
use crate::{config::smtp::SmtpConfig, smtp::client::SmtpClient};

/// What a send did with the copy it was asked to file.
#[cfg_attr(not(any(feature = "gmail", feature = "pimdir")), allow(dead_code))]
pub enum SentCopy {
    /// Not filed: the caller saves it.
    Owed,
    /// Filed by the backend as part of the send, under the id it reported.
    Filed(Option<String>),
}

/// The client every unified subcommand runs over.
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
    /// The credentials of the run, so a send reuses those the storage ran.
    #[cfg(feature = "smtp")]
    resolver: SecretResolver,
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
        let mut resolver = SecretResolver::new();
        let storage = open_storage(&mut account_config, resolution.storage, &mut resolver)?;

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
            // NOTE: ManageSieve holds no mailbox, so no unified command runs
            // on it.
            let supported: Vec<Backend> = Backend::COMPILED
                .iter()
                .copied()
                .filter(|b| *b != Backend::Sieve)
                .collect();

            return Err(account_config.no_backend_error(name, backend, &supported));
        }

        let account = Account {
            name: Some(name.to_string()),
            ..Account::from(config).merge(Account::from(account_config))
        };

        Ok((
            account,
            Self {
                resolution,
                storage,
                #[cfg(feature = "smtp")]
                smtp,
                #[cfg(feature = "smtp")]
                resolver,
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
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client.list_mailboxes(with_counts),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.list_mailboxes(with_counts),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Creates the mailbox `name` at the top, or under `parent`,
    /// returning its id, `None` when the backend queues the creation, and
    /// its full name where the backend names the hierarchy by path.
    pub fn create_mailbox(
        &mut self,
        name: &str,
        parent: Option<&str>,
    ) -> Result<(Option<String>, String)> {
        self.require(&[Capability::MailboxCreate])?;
        mailbox::validate_name(name)?;

        let parent = match parent {
            Some(parent) => Some(self.resolve_mailbox_id(parent)?),
            None => None,
        };
        let parent = parent.as_deref();

        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.create_mailbox(name, parent).map(named),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client
                .create_mailbox(name, parent)
                .map(|id| (Some(id), name.into())),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client
                .create_mailbox(name, parent)
                .map(|id| (Some(id), name.into())),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client
                .create_mailbox(name, parent)
                .map(|id| (Some(id), name.into())),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client
                .create_mailbox(name, parent)
                .map(|(id, name)| (Some(id), name)),
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => {
                mailbox::validate_level(name, '/')?;
                client.create_mailbox(name, parent).map(named)
            }
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client
                .create_mailbox(name, parent)
                .map(|_| (None, name.into())),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Renames the last level of `mailbox` to `name`, its parent kept,
    /// returning the id it has afterwards, `None` when the backend queues
    /// the rename, and its full name where the backend names the hierarchy
    /// by path.
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
    pub fn rename_mailbox(
        &mut self,
        mailbox: &str,
        name: &str,
    ) -> Result<(Option<String>, String)> {
        self.require(&[Capability::MailboxRename])?;
        mailbox::validate_name(name)?;

        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.rename_mailbox(mailbox, name).map(named),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client
                .rename_mailbox(mailbox, name)
                .map(|id| (Some(id), name.into())),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client
                .rename_mailbox(mailbox, name)
                .map(|id| (Some(id), name.into())),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client
                .rename_mailbox(mailbox, name)
                .map(|id| (Some(id), name.into())),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client
                .rename_mailbox(mailbox, name)
                .map(|(id, name)| (Some(id), name)),
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => {
                mailbox::validate_level(name, '/')?;
                client.rename_mailbox(mailbox, name).map(named)
            }
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client
                .rename_mailbox(mailbox, name)
                .map(|_| (None, name.into())),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Deletes `mailbox` and the messages it holds, returning its id, the
    /// one `mailbox list` prints.
    #[cfg_attr(not(backend), allow(unreachable_code))]
    pub fn delete_mailbox(&mut self, mailbox: &str) -> Result<String> {
        self.require(&[Capability::MailboxDelete])?;

        let mailbox = self.resolve_mailbox_id(mailbox)?;
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.delete_mailbox(&mailbox).map(|()| mailbox),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.delete_mailbox(&mailbox).map(|()| mailbox),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.delete_mailbox(&mailbox).map(|()| mailbox),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.delete_mailbox(&mailbox).map(|()| mailbox),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.delete_mailbox(&mailbox),
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client.delete_mailbox(&mailbox).map(|()| mailbox),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.delete_mailbox(&mailbox).map(|_| mailbox),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
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
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client.get_message(mailbox, id, seen),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.get_message(mailbox, id, seen),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Adds a raw message to a mailbox with the given flags, refusing a
    /// flag the backend cannot keep. Returns the new message id, absent
    /// when an IMAP or JMAP server does not report it.
    #[cfg_attr(
        not(any(
            feature = "imap",
            feature = "jmap",
            feature = "gmail",
            feature = "maildir",
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
        let mut capabilities = vec![Capability::MessageAdd];
        capabilities.extend(flags.iter().filter_map(Flag::capability));
        self.require(&capabilities)?;

        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.add_message(mailbox, flags, raw),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.add_message(mailbox, flags, raw),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.add_message(mailbox, flags, raw).map(Some),
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
    /// Maildir none, the caller falling back on the
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
            BackendClient::Imap(client) => client.role_mailbox_id(role),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.role_mailbox_id(role),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.role_mailbox_id(role),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.role_mailbox_id(role),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(_) => Ok(None),
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
    /// Returns how many of the ids the mailbox held, every id for a
    /// backend that cannot tell, and whether they were really removed, an
    /// IMAP server without UIDPLUS only flagging them `\Deleted`.
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
    pub fn delete_messages(&mut self, mailbox: &str, ids: &[&str]) -> Result<(usize, bool)> {
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.delete_messages(mailbox, ids),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.delete_messages(mailbox, ids).map(|n| (n, true)),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.delete_messages(mailbox, ids).map(|n| (n, true)),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => {
                client.delete_messages(mailbox, ids).map(|n| (n, true))
            }
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client
                .delete_messages(mailbox, ids)
                .map(|found| (found, true)),
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client.delete_messages(mailbox, ids).map(|n| (n, true)),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client
                .delete_messages(mailbox, ids)
                .map(|count| (count, true)),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Refuses the first of `capabilities` the account does not serve,
    /// before anything is sent.
    pub fn require(&self, capabilities: &[Capability]) -> Result<()> {
        self.resolution.require(capabilities)
    }

    /// The account the client runs for and the backends it resolved to.
    pub fn resolution(&self) -> &Resolution {
        &self.resolution
    }

    /// The flags as the storage backend writes them, which a receipt
    /// names: JMAP spells an IANA flag as its keyword (`$flagged`), the
    /// other backends as given.
    pub fn written_flags(&self, flags: Vec<Flag>) -> Vec<Flag> {
        match &self.storage {
            #[cfg(feature = "jmap")]
            Some(BackendClient::Jmap(_)) => flags
                .iter()
                .map(crate::jmap::backend::written_flag)
                .collect(),
            _ => flags,
        }
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

    /// Whether the storage backend files every message it sends in its sent
    /// mailbox itself, whatever is asked: Microsoft Graph does, its
    /// `sendMail` saving to Sent Items.
    pub fn files_sent_copy(&self) -> bool {
        #[cfg(feature = "msgraph")]
        if let Some(BackendClient::Msgraph(_)) = &self.storage {
            return true;
        }
        false
    }

    /// Sends a raw message through the storage backend when it can send,
    /// and through the SMTP transport otherwise.
    ///
    /// `mailbox` is the sent mailbox, and `copy` asks for the message to
    /// be filed there with `flags`. A backend may file that copy itself as
    /// part of the send, [`SentCopy::Filed`]: pimdir, and Gmail, which
    /// files every sent message under `SENT` and labels that one rather
    /// than insert a second message differing from the one delivered. The
    /// others owe the copy to the caller.
    #[cfg_attr(not(feature = "gmail"), allow(unused_variables))]
    pub fn send_message(
        &mut self,
        mailbox: Option<&str>,
        flags: &[Flag],
        raw: Vec<u8>,
        copy: bool,
    ) -> Result<SentCopy> {
        match &mut self.storage {
            #[cfg(feature = "jmap")]
            Some(BackendClient::Jmap(client)) => {
                return client.send_message(raw).map(|()| SentCopy::Owed);
            }
            #[cfg(feature = "gmail")]
            Some(BackendClient::Gmail(client)) => {
                let id = client.send_message(raw)?;
                let Some(mailbox) = mailbox.filter(|_| copy) else {
                    return Ok(SentCopy::Owed);
                };

                let context = || format!("Message sent, but saving a copy to {mailbox} failed");
                let label = client.resolve_mailbox_id(mailbox).with_context(context)?;
                client.file_sent(&id, &label, flags).with_context(context)?;
                return Ok(SentCopy::Filed(Some(id)));
            }
            #[cfg(feature = "msgraph")]
            // NOTE: Graph files every message it sends in Sent Items itself.
            Some(BackendClient::Msgraph(client)) => {
                return client.send_message(raw).map(|()| SentCopy::Filed(None));
            }
            #[cfg(feature = "pimdir")]
            Some(BackendClient::Pimdir(client)) => {
                let sent = client.send_message(mailbox, raw, copy)?;
                return Ok(match sent.carried {
                    true => SentCopy::Filed(None),
                    false => SentCopy::Owed,
                });
            }
            _ => {}
        }

        #[cfg(feature = "smtp")]
        if let Some(config) = &self.smtp {
            let mut smtp = SmtpClient::new(config.as_ref().clone(), &mut self.resolver)?;
            return smtp.send_message(raw).map(|()| SentCopy::Owed);
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
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.resolve_mailbox_id(mailbox),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.resolve_mailbox_id(mailbox),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.resolve_mailbox_id(mailbox),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.resolve_mailbox_id(mailbox),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client.resolve_mailbox_id(mailbox),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.resolve_mailbox_id(mailbox),
            #[allow(unreachable_patterns)]
            _ => Ok(mailbox.to_string()),
        }
    }

    /// The id a role alias names on a backend taking several spellings
    /// of one mailbox, `None` elsewhere or when it names none.
    ///
    /// Only Maildir resolves, locally: the other backends compare the
    /// alias with the listed ids and names as it is.
    pub fn alias_mailbox_id(&mut self, alias: &str) -> Option<String> {
        match self.storage.as_mut()? {
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.resolve_mailbox_id(alias).ok(),
            #[allow(unreachable_patterns)]
            _ => {
                let _ = alias;
                None
            }
        }
    }

    fn storage_mut(&mut self) -> Result<&mut BackendClient> {
        self.storage
            .as_mut()
            .ok_or_else(|| anyhow!("No storage backend is configured for this account"))
    }
}

/// The id and full name of a mailbox on a backend whose id is its full
/// name (IMAP, mbox).
#[cfg(any(feature = "imap", feature = "mbox"))]
fn named(id: String) -> (Option<String>, String) {
    (Some(id.clone()), id)
}

/// Opens the storage backend the resolver picked, taking its block out
/// of the account.
#[cfg_attr(
    not(any(
        feature = "maildir",
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
    #[cfg_attr(not(feature = "imap"), allow(unused_variables))] resolver: &mut SecretResolver,
) -> Result<Option<BackendClient>> {
    let client = match storage {
        #[cfg(feature = "maildir")]
        Some(Backend::Maildir) => account_config
            .maildir
            .take()
            .map(|config| BackendClient::Maildir(Box::new(MaildirClient::new(config)))),
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
            Some(config) => Some(BackendClient::Imap(Box::new(ImapClient::new(
                config, resolver,
            )?))),
            None => None,
        },
        _ => None,
    };

    Ok(client)
}

#[cfg(all(test, feature = "imap", feature = "smtp"))]
mod tests {
    use std::{
        fs,
        io::{BufRead, BufReader, Write},
        net::TcpListener,
        thread,
    };

    use super::*;

    /// Serves one IMAP session that accepts any credential.
    fn serve_imap() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut out = stream;
            write!(out, "* OK [CAPABILITY IMAP4rev1 AUTH=PLAIN SASL-IR] hi\r\n").unwrap();

            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                let tag = line.split(' ').next().unwrap_or_default().to_owned();
                let reply = match line.split(' ').nth(1).map(str::trim) {
                    Some("CAPABILITY") => {
                        format!("* CAPABILITY IMAP4rev1\r\n{tag} OK done\r\n")
                    }
                    Some("LOGOUT") => format!("* BYE bye\r\n{tag} OK done\r\n"),
                    _ => format!("{tag} OK done\r\n"),
                };

                line.clear();
                if out.write_all(reply.as_bytes()).is_err() {
                    break;
                }
            }
        });

        format!("imap://{addr}")
    }

    #[test]
    fn a_send_reuses_the_credential_the_storage_resolved() {
        let dir = tempfile::tempdir().unwrap();
        let runs = dir.path().join("runs");
        let command = format!("echo run >> '{}'; echo secret", runs.display());

        // NOTE: nothing listens there, so the send fails once its
        // credential is resolved.
        let closed = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();

        let account_config: AccountConfig = toml::from_str(&format!(
            "imap.server = \"{imap}\"\n\
             imap.sasl.plain.username = \"alice\"\n\
             imap.sasl.plain.password.command = \"{command}\"\n\
             smtp.server = \"smtp://{closed}\"\n\
             smtp.sasl.plain.username = \"alice\"\n\
             smtp.sasl.plain.password.command = \"{command}\"\n",
            imap = serve_imap(),
        ))
        .unwrap();

        let (_, mut client) =
            EmailClient::new(Config::default(), "alice", account_config, Backend::Auto).unwrap();

        let raw = b"From: a@x.org\r\nTo: b@x.org\r\n\r\nhi\r\n".to_vec();
        assert!(client.send_message(None, &[], raw, false).is_err());
        assert_eq!(fs::read_to_string(&runs).unwrap(), "run\n");
    }
}

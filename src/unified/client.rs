//! # Email client
//!
//! The cross-protocol client every unified subcommand runs over: one
//! storage backend, plus an SMTP transport for the backends that cannot
//! send.
//!
//! The storage backend is the one [`Resolution`] picks: the first
//! configured one `--backend` allows, local before network. Each method
//! matches on it and calls that protocol's adapter, which takes and returns
//! the CLI's own [`crate::unified::email`] types.
//!
//! The active [`Account`] is threaded as a sibling argument through every
//! `execute` chain rather than bundled into the client.

#![cfg_attr(not(backend), allow(unused_variables))]

use std::collections::BTreeSet;

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
    config::{AccountConfig, Config, sasl::SaslConfig},
    unified::{
        backend::Backend,
        capability::{Capability, Resolution},
        email::{
            envelope::EnvelopePage,
            flag::{Flag, FlagOp},
            mailbox::{self, Mailbox, MailboxRole},
            outcome::Outcome,
            search::query::SearchEmailsQuery,
        },
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
    /// Not filed, the backend failing to file it after the message went
    /// out.
    #[cfg_attr(not(feature = "gmail"), allow(dead_code))]
    Unfiled(anyhow::Error),
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
    ///
    /// SMTP is kept as configuration, unconnected, so a read-only command
    /// opens no SMTP connection.
    pub fn new(
        config: Config,
        name: &str,
        #[allow(unused_mut)] mut account_config: AccountConfig,
        backend: Backend,
    ) -> Result<(Account, Self)> {
        let resolution = Resolution::of(name, &account_config, backend);
        let logins = logins(&account_config);
        let mut resolver = SecretResolver::new();
        let storage = open_storage(&mut account_config, resolution.storage, &mut resolver)?;

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
            let supported: Vec<Backend> = Backend::COMPILED
                .iter()
                .copied()
                .filter(|b| *b != Backend::Sieve)
                .collect();

            return Err(account_config.no_backend_error(name, backend, &supported));
        }

        let account = Account {
            name: Some(name.to_string()),
            logins,
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

    /// Lists every mailbox available to the active account, beside the
    /// mailboxes it could not read.
    pub fn list_mailboxes(&mut self, with_counts: bool) -> Result<Outcome<Vec<Mailbox>>> {
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
            BackendClient::Pimdir(client) => client.list_mailboxes(with_counts).map(Outcome::from),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Creates `name` at the top or under `parent`: its id (`None` when
    /// queued) and its full name where the hierarchy is named by path.
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

    /// Renames the last level of `mailbox` to `name`: its new id (`None`
    /// when queued) and its full name where the hierarchy is named by path.
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

    /// Lists one page of envelopes from `mailbox`, with its message count
    /// when the listing's own exchanges answer it.
    ///
    /// `None` otherwise: Gmail's count is an estimate, Graph's a request.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<(Outcome<EnvelopePage>, Option<u64>)> {
        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => {
                client.list_envelopes_counted(mailbox, page, page_size, with_attachment)
            }
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => {
                client.list_envelopes_counted(mailbox, page, page_size, with_attachment)
            }
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client
                .list_envelopes(mailbox, page, page_size, with_attachment)
                .map(|envelopes| (envelopes, None)),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client
                .list_envelopes(mailbox, page, page_size, with_attachment)
                .map(|envelopes| (Outcome::from(envelopes), None)),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => {
                client.list_envelopes_counted(mailbox, page, page_size, with_attachment)
            }
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => {
                client.list_envelopes_counted(mailbox, page, page_size, with_attachment)
            }
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client
                .list_envelopes_counted(mailbox, page, page_size, with_attachment)
                .map(|(envelopes, total)| (Outcome::from(envelopes), total)),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Searches a mailbox with the unified query, one page of it.
    pub fn search_envelopes(
        &mut self,
        mailbox: &str,
        query: Option<&SearchEmailsQuery>,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<Outcome<EnvelopePage>> {
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
            BackendClient::Pimdir(client) => client
                .search_envelopes(mailbox, query, page, page_size, with_attachment)
                .map(Outcome::from),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => {
                client.search_envelopes(mailbox, query, page, page_size, with_attachment)
            }
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client
                .search_envelopes(mailbox, query, page, page_size, with_attachment)
                .map(Outcome::from),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Adds, sets or removes flags on a set of messages, returning how
    /// many of them were found beside the error of each one not acted on.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<Outcome<usize>> {
        let capabilities: Vec<Capability> = flags.iter().filter_map(Flag::capability).collect();
        self.require(&capabilities)?;

        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();
        let outcome: Outcome<_> = match self.storage_mut()? {
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
            _ => Err(anyhow!("No storage backend is configured for this account")),
        }?;

        Ok(outcome.sorted_by(ids))
    }

    /// Fetches one message's raw RFC 5322 bytes and its flags, marking
    /// it seen when asked.
    ///
    /// IMAP folds the flag into the fetch, the others issue an update. JMAP,
    /// Gmail and Graph resolve the mailbox only when the message is not
    /// where it was named.
    pub fn get_message(
        &mut self,
        mailbox: &str,
        id: &str,
        seen: bool,
    ) -> Result<(Vec<u8>, BTreeSet<Flag>)> {
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

    /// Adds a raw message with `flags`, refusing one the backend cannot
    /// keep: the new id, absent when an IMAP or JMAP server omits it.
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
                .map(|staged| Some(staged.link_id)),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.add_message(mailbox, flags, raw).map(Some),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(_) => bail!("Microsoft Graph does not support adding messages"),
            #[cfg(not(backend))]
            _ => bail!("No storage backend is configured for this account"),
        }
    }

    /// Copies messages between two mailboxes, returning how many landed,
    /// beside the error of each message not acted on.
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let from = self.resolve_mailbox_id(from)?;
        let to = self.resolve_mailbox_id(to)?;
        let (from, to) = (from.as_str(), to.as_str());
        let outcome: Outcome<_> = match self.storage_mut()? {
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
            _ => Err(anyhow!("No storage backend is configured for this account")),
        }?;

        Ok(outcome.sorted_by(ids))
    }

    /// Moves messages between two mailboxes, returning how many landed,
    /// beside the error of each message not acted on.
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let from = self.resolve_mailbox_id(from)?;
        let to = self.resolve_mailbox_id(to)?;
        let (from, to) = (from.as_str(), to.as_str());
        let outcome: Outcome<_> = match self.storage_mut()? {
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
            _ => Err(anyhow!("No storage backend is configured for this account")),
        }?;

        Ok(outcome.sorted_by(ids))
    }

    /// The id of the mailbox the backend marks with `role`, `None` when it
    /// marks none.
    ///
    /// IMAP knows only `INBOX` until SPECIAL-USE is read, mbox the spool as
    /// `INBOX`, Maildir none: the caller falls back on
    /// `mailbox.alias.<role>`.
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
    pub fn delete_messages(
        &mut self,
        mailbox: &str,
        ids: &[&str],
    ) -> Result<Outcome<(usize, bool)>> {
        let outcome: Outcome<_> = match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.delete_messages(mailbox, ids),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client
                .delete_messages(mailbox, ids)
                .map(|outcome| outcome.map(|n| (n, true))),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client
                .delete_messages(mailbox, ids)
                .map(|outcome| outcome.map(|n| (n, true))),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client
                .delete_messages(mailbox, ids)
                .map(|outcome| outcome.map(|n| (n, true))),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client
                .delete_messages(mailbox, ids)
                .map(|outcome| outcome.map(|n| (n, true))),
            #[cfg(feature = "mbox")]
            BackendClient::Mbox(client) => client
                .delete_messages(mailbox, ids)
                .map(|outcome| outcome.map(|n| (n, true))),
            #[cfg(feature = "pimdir")]
            BackendClient::Pimdir(client) => client
                .delete_messages(mailbox, ids)
                .map(|outcome| outcome.map(|n| (n, true))),
            #[cfg(not(backend))]
            _ => Err(anyhow!("No storage backend is configured for this account")),
        }?;

        Ok(outcome.sorted_by(ids))
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

    /// The flags as the backend writes them, for a receipt: JMAP spells an
    /// IANA flag as its keyword (`$flagged`), the others as given.
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

    /// Whether the backend queues writes for a sync engine, so a
    /// confirmation says queued rather than done.
    pub fn queues_writes(&self) -> bool {
        #[cfg(feature = "pimdir")]
        if let Some(BackendClient::Pimdir(_)) = &self.storage {
            return true;
        }
        false
    }

    /// Whether the backend files every sent message itself, whatever is
    /// asked: Graph's `sendMail` saves to Sent Items.
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
    /// `copy` files it in the sent `mailbox` with `flags`. pimdir and Gmail
    /// file it themselves ([`SentCopy::Filed`]), Gmail labelling its `SENT`
    /// copy rather than insert a second, different one; others leave it.
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
                let filed = client
                    .resolve_mailbox_id(mailbox)
                    .and_then(|label| client.file_sent(&id, &label, flags));

                return Ok(match filed {
                    Ok(()) => SentCopy::Filed(Some(id)),
                    Err(err) => SentCopy::Unfiled(err.context(context())),
                });
            }
            #[cfg(feature = "msgraph")]
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
    /// Every dispatching method runs it first, so adapters only see ids.
    /// Idempotent, an id passing through unchanged, which lets a caller
    /// compare a mailbox against the trash.
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

    /// Keeps the storage connection alive: `NOOP` on IMAP, nothing for a
    /// backend holding no connection between requests.
    pub fn keep_alive(&mut self) -> Result<()> {
        match self.storage.as_mut() {
            #[cfg(feature = "imap")]
            Some(BackendClient::Imap(client)) => client.keep_alive(),
            _ => Ok(()),
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

/// The addresses the account logs in as, each once: the IMAP and SMTP SASL
/// user names holding an `@` (for PLAIN the acted-as identity too).
fn logins(account_config: &AccountConfig) -> Vec<String> {
    let blocks = [
        account_config.imap.as_ref().and_then(|c| c.sasl.as_ref()),
        account_config.smtp.as_ref().and_then(|c| c.sasl.as_ref()),
    ];
    let mut logins: Vec<String> = Vec::new();

    for sasl in blocks.into_iter().flatten() {
        let names = match sasl {
            SaslConfig::Anonymous(_) => vec![],
            SaslConfig::Login(config) => vec![Some(&config.username)],
            SaslConfig::Plain(config) => vec![Some(&config.authcid), config.authzid.as_ref()],
            SaslConfig::Oauthbearer(config) => vec![Some(&config.username)],
            SaslConfig::Xoauth2(config) => vec![Some(&config.username)],
            SaslConfig::ScramSha256(config) => vec![Some(&config.username)],
        };
        for name in names.into_iter().flatten() {
            if name.contains('@') && !logins.contains(name) {
                logins.push(name.clone());
            }
        }
    }

    logins
}

#[cfg(test)]
mod logins_tests {
    use pimalaya_config::toml::TomlConfig;

    use crate::config::Config;

    /// An account's logins are its IMAP and SMTP SASL user names reading
    /// as addresses, each once.
    #[test]
    fn the_logins_are_the_sasl_names_holding_an_address() {
        let toml = "[accounts.icloud]\n\
                    imap.server = \"imap.example.org\"\n\
                    imap.sasl.plain.username = \"me@example.org\"\n\
                    imap.sasl.plain.password.raw = \"x\"\n\
                    imap.sasl.plain.authzid = \"shared@example.org\"\n\
                    smtp.server = \"smtp.example.org\"\n\
                    smtp.sasl.login.username = \"me@example.org\"\n\
                    smtp.sasl.login.password.raw = \"x\"\n\
                    [accounts.local]\n\
                    imap.server = \"imap.example.org\"\n\
                    imap.sasl.login.username = \"me\"\n\
                    imap.sasl.login.password.raw = \"x\"\n";

        let mut config: Config = toml::from_str(toml).unwrap();
        let (_, icloud) = config.take_account(Some("icloud")).unwrap();
        assert_eq!(
            super::logins(&icloud),
            ["me@example.org", "shared@example.org"]
        );
        let (_, local) = config.take_account(Some("local")).unwrap();
        assert!(super::logins(&local).is_empty());
    }
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

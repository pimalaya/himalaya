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
#[cfg(feature = "smtp")]
use crate::smtp::client::SmtpClient;
use crate::{
    account::context::Account,
    backend::Backend,
    config::{AccountConfig, Config},
    email::{
        envelope::Envelope,
        flag::{Flag, FlagOp},
        mailbox::Mailbox,
        search::query::SearchEmailsQuery,
    },
};

/// Cross-protocol email client backing the shared subcommands.
pub struct EmailClient {
    storage: Option<BackendClient>,
    #[cfg(feature = "smtp")]
    smtp: Option<SmtpClient>,
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

        #[cfg(feature = "smtp")]
        let smtp = match (backend.allows_smtp(), account_config.smtp.take()) {
            (true, Some(config)) => Some(SmtpClient::new(config)?),
            _ => None,
        };

        #[cfg(feature = "smtp")]
        let has_transport = storage.is_some() || smtp.is_some();
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
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
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
    pub fn search_envelopes(
        &mut self,
        mailbox: &str,
        query: Option<&SearchEmailsQuery>,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
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
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
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

    /// Fetches one message's raw RFC 5322 bytes.
    pub fn get_message(&mut self, mailbox: &str, id: &str) -> Result<Vec<u8>> {
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.get_message(mailbox, id),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.get_message(mailbox, id),
            #[cfg(feature = "gmail")]
            BackendClient::Gmail(client) => client.get_message(mailbox, id),
            #[cfg(feature = "msgraph")]
            BackendClient::Msgraph(client) => client.get_message(mailbox, id),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.get_message(mailbox, id),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.get_message(mailbox, id),
        }
    }

    /// Adds `raw` to `mailbox` with `flags`. Gmail and Microsoft Graph
    /// do not implement adding messages.
    pub fn add_message(&mut self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
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
        }
    }

    /// Copies a message id set from `from` to `to`.
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<()> {
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

    /// Moves a message id set from `from` to `to`.
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<()> {
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
        if let Some(smtp) = &mut self.smtp {
            return smtp.send_message(raw);
        }

        bail!("No send-capable backend (JMAP/Gmail/Graph) or SMTP is configured for this account")
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
    if backend.allows_maildir() {
        if let Some(config) = account_config.maildir.take() {
            return Ok(Some(BackendClient::Maildir(Box::new(MaildirClient::new(
                config,
            )))));
        }
    }

    #[cfg(feature = "m2dir")]
    if backend.allows_m2dir() {
        if let Some(config) = account_config.m2dir.take() {
            return Ok(Some(BackendClient::M2dir(Box::new(M2dirClient::new(
                config,
            )))));
        }
    }

    #[cfg(feature = "jmap")]
    if backend.allows_jmap() {
        if let Some(config) = account_config.jmap.take() {
            return Ok(Some(BackendClient::Jmap(Box::new(JmapClient::new(
                config,
            )?))));
        }
    }

    #[cfg(feature = "gmail")]
    if backend.allows_gmail() {
        if let Some(config) = account_config.gmail.take() {
            return Ok(Some(BackendClient::Gmail(Box::new(GmailClient::new(
                config,
            )?))));
        }
    }

    #[cfg(feature = "msgraph")]
    if backend.allows_msgraph() {
        if let Some(config) = account_config.msgraph.take() {
            return Ok(Some(BackendClient::Msgraph(Box::new(MsgraphClient::new(
                config,
            )?))));
        }
    }

    #[cfg(feature = "imap")]
    if backend.allows_imap() {
        if let Some(config) = account_config.imap.take() {
            return Ok(Some(BackendClient::Imap(Box::new(ImapClient::new(
                config,
            )?))));
        }
    }

    Ok(None)
}

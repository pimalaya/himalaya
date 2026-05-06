//! Cross-protocol [`EmailClient`] for the shared subcommands
//! (`mailboxes`, `envelopes`, `flags`, `messages`, `attachments`).
//!
//! Wraps [`io_email::client::EmailClient`] and bundles the active
//! [`Account`] (display, identity, composer/reader registries) the
//! shared commands need alongside the I/O client. Implements
//! [`Deref`]/[`DerefMut`] onto the inner client so callers can call
//! its methods directly.
//!
//! Construction is backend-asymmetric (IMAP needs TLS + SASL, JMAP
//! needs an HTTP credential, Maildir just needs a root path). Each
//! `new_<protocol>` constructor delegates to the transitional
//! [`ImapSession`] / [`JmapSession`] helpers for the handshake/auth
//! flow then bridges the resulting `(stream, context)` pairs into
//! [`io_imap::client::ImapClient`] / [`io_jmap::client::JmapClient`]
//! via their `from_parts` constructors.
//!
//! [`ImapSession`]: crate::imap::session::ImapSession
//! [`JmapSession`]: crate::jmap::session::JmapSession

use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use anyhow::{anyhow, bail, Result};
use io_email::client::SendMessageOpts;
use pimalaya_config::toml::TomlConfig;

use crate::{
    account::context::Account,
    cli::{load_or_wizard, BackendFlag},
};

pub struct EmailClient {
    inner: io_email::client::EmailClient,
    pub account: Account,
    /// Pre-computed options for [`io_email::client::EmailClient::send_message`].
    /// Populated by the per-protocol constructors with the bits each
    /// backend needs (currently only the JMAP identity / drafts
    /// mailbox ids); other fields are filled in at send time from the
    /// outgoing message itself.
    pub send_opts: SendMessageOpts,
}

impl EmailClient {
    /// Loads the configuration, picks the active account, builds the
    /// merged [`Account`], then constructs an [`EmailClient`] for the
    /// first backend allowed by `backend` that is configured on the
    /// account. Selection order is `jmap → imap → maildir`. Bails when
    /// no backend matches.
    pub fn new(
        config_paths: &[PathBuf],
        account_name: Option<&str>,
        backend: BackendFlag,
    ) -> Result<Self> {
        let mut config = load_or_wizard(config_paths)?;
        let (_, mut ac) = config
            .take_account(account_name)?
            .ok_or_else(|| anyhow!("Cannot find account"))?;

        #[cfg(feature = "jmap")]
        if backend.allows_jmap() {
            if let Some(jmap_config) = ac.jmap.take() {
                let account = Account::from(config).merge(Account::from(ac));
                return EmailClient::new_jmap(jmap_config, account);
            }
        }

        #[cfg(feature = "imap")]
        if backend.allows_imap() {
            if let Some(imap_config) = ac.imap.take() {
                let account = Account::from(config).merge(Account::from(ac));
                return EmailClient::new_imap(imap_config, account);
            }
        }

        #[cfg(feature = "maildir")]
        if backend.allows_maildir() {
            if let Some(maildir_config) = ac.maildir.take() {
                let account = Account::from(config).merge(Account::from(ac));
                return EmailClient::new_maildir(maildir_config, account);
            }
        }

        bail!("no backend matching `{backend}` is configured for this account")
    }

    #[cfg(feature = "imap")]
    pub fn new_imap(config: crate::config::ImapConfig, account: Account) -> Result<Self> {
        use io_imap::client::ImapClient;

        use crate::imap::session::ImapSession;

        let session = ImapSession::new(
            config.url,
            config.tls.try_into()?,
            config.starttls,
            config.sasl.try_into()?,
        )?;
        let client = ImapClient::from_parts(session.stream, session.context);
        Ok(Self {
            inner: client.into(),
            account,
            send_opts: SendMessageOpts::default(),
        })
    }

    #[cfg(feature = "jmap")]
    pub fn new_jmap(config: crate::config::JmapConfig, account: Account) -> Result<Self> {
        use io_jmap::client::JmapClient;

        use crate::jmap::session::JmapSession;

        let send_opts = SendMessageOpts {
            jmap_identity_id: config.identity_id.clone(),
            jmap_drafts_mailbox_id: config.drafts_mailbox_id.clone(),
            ..SendMessageOpts::default()
        };

        let session = JmapSession::new(
            config.server,
            config.tls.try_into()?,
            config.auth.try_into()?,
        )?;
        let client = JmapClient::from_parts(session.stream, session.http_auth, session.session);
        Ok(Self {
            inner: client.into(),
            account,
            send_opts,
        })
    }

    #[cfg(feature = "maildir")]
    pub fn new_maildir(config: crate::config::MaildirConfig, account: Account) -> Result<Self> {
        use io_maildir::client::MaildirClient;

        let client = MaildirClient::new(config.root);
        Ok(Self {
            inner: client.into(),
            account,
            send_opts: SendMessageOpts::default(),
        })
    }
}

impl Deref for EmailClient {
    type Target = io_email::client::EmailClient;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for EmailClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

//! Cross-protocol [`EmailClient`] for the shared subcommands
//! (`mailboxes`, `envelopes`, `flags`, `messages`, `attachments`).
//!
//! Wraps [`io_email::client::EmailClientStd`] and bundles the active
//! [`Account`] (display, identity, composer/reader registries) the
//! shared commands need alongside the I/O client. Implements
//! [`Deref`]/[`DerefMut`] onto the inner client so callers can call
//! its methods directly.
//!
//! Construction is backend-asymmetric (IMAP needs TLS + SASL, JMAP
//! needs an HTTP credential, Maildir just needs a root path). The
//! single [`EmailClient::new`] entry-point loads the configuration,
//! picks the merged account, then walks the configured backends in
//! `jmap → imap → maildir` order and opens the first one allowed by
//! the `BackendFlag`.

use std::ops::{Deref, DerefMut};

use anyhow::{bail, Result};

use crate::{
    account::context::Account,
    backend::Backend,
    config::{AccountConfig, Config},
};

pub struct EmailClient {
    inner: io_email::client::EmailClientStd,
    pub account: Account,
}

impl EmailClient {
    /// Loads the configuration, picks the active account, builds the
    /// merged [`Account`], then opens the first backend allowed by
    /// `backend` that is configured on the account. Selection order
    /// is `jmap → imap → maildir`. Bails when no backend matches.
    pub fn new(
        config: Config,
        mut account_config: AccountConfig,
        backend: Backend,
    ) -> Result<Self> {
        #[cfg(feature = "jmap")]
        if backend.allows_jmap() {
            if let Some(jmap_config) = account_config.jmap.take() {
                use io_email::client::EmailClientStd;
                use io_jmap::client::JmapClientStd;
                use pimalaya_stream::tls::Tls;

                use crate::jmap::client::{jmap_http_auth, parse_server_url};

                let account = Account::from(config).merge(Account::from(account_config));
                let mut tls: Tls = jmap_config.tls.clone().into();
                tls.rustls.alpn = vec!["http/1.1".into()];
                let http_auth = jmap_http_auth(jmap_config.auth.clone())?;
                let url = parse_server_url(&jmap_config.server)?;
                let mut client = JmapClientStd::connect(&url, &tls, http_auth)?;
                client.session_get(&url)?;

                return Ok(Self {
                    inner: EmailClientStd::Jmap(client),
                    account,
                });
            }
        }

        #[cfg(feature = "imap")]
        if backend.allows_imap() {
            if let Some(imap_config) = account_config.imap.take() {
                use io_email::client::EmailClientStd;
                use io_imap::client::ImapClientStd;
                use pimalaya_stream::{sasl::Sasl, std::stream::StreamStd, tls::Tls};

                let account = Account::from(config).merge(Account::from(account_config));
                let mut tls: Tls = imap_config.tls.into();
                tls.rustls.alpn = vec!["imap".into()];
                let sasl: Option<Sasl> = imap_config.sasl.map(Sasl::try_from).transpose()?;
                let server = crate::imap::client::parse_imap_server(&imap_config.server)?;
                let client =
                    ImapClientStd::<StreamStd>::connect(&server, &tls, imap_config.starttls, sasl)?;

                return Ok(Self {
                    inner: EmailClientStd::Imap(client),
                    account,
                });
            }
        }

        #[cfg(feature = "maildir")]
        if backend.allows_maildir() {
            if let Some(maildir_config) = account_config.maildir.take() {
                use io_email::client::EmailClientStd;
                use io_maildir::client::MaildirClient;

                let account = Account::from(config).merge(Account::from(account_config));
                let client = MaildirClient::new(maildir_config.root);

                return Ok(Self {
                    inner: EmailClientStd::Maildir(client),
                    account,
                });
            }
        }

        bail!("no backend matching `{backend}` is configured for this account")
    }
}

impl Deref for EmailClient {
    type Target = io_email::client::EmailClientStd;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for EmailClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

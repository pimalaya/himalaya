//! Cross-protocol [`EmailClient`] for the shared subcommands
//! (`mailboxes`, `envelopes`, `flags`, `messages`, `attachments`).
//!
//! Wraps [`io_email::client::EmailClientStd`]. The active
//! [`Account`] is threaded as a sibling argument through every
//! `execute` chain rather than being bundled into the client; this
//! keeps account access (`resolve_mailbox`, identity, etc.) borrow-
//! disjoint from `&mut EmailClient` calls. Implements
//! [`Deref`]/[`DerefMut`] onto the inner client so callers can call
//! its methods directly.
//!
//! Construction registers every storage backend (`jmap`, `gmail`,
//! `msgraph`, `imap`, `maildir`, `m2dir`) that the [`Backend`] flag
//! allows and the account has configured; io-email's dispatcher then
//! routes each shared call to the appropriate one by priority. When
//! the account also has SMTP configured, an SMTP slot is registered
//! too so `send_message` works for IMAP/Maildir accounts; JMAP
//! accounts send via JMAP submission. A connection failure on any
//! registered backend (SMTP included) aborts construction.

use std::ops::{Deref, DerefMut};

use anyhow::Result;
use io_email::client::EmailClientStd;

use crate::{
    account::context::Account,
    backend::Backend,
    config::{AccountConfig, Config},
};

/// Cross-protocol email client backing the shared subcommands.
pub struct EmailClient {
    inner: EmailClientStd,
}

impl EmailClient {
    pub fn new(
        config: Config,
        mut account_config: AccountConfig,
        backend: Backend,
    ) -> Result<(Account, Self)> {
        let mut inner = EmailClientStd::new();

        #[cfg(feature = "jmap")]
        if backend.allows_jmap() {
            if let Some(jmap_config) = account_config.jmap.take() {
                use crate::jmap::client::{jmap_http_auth, parse_server_url};

                let tls = jmap_config.tls.clone().into_tls(jmap_config.alpn.clone());
                let http_auth = jmap_http_auth(jmap_config.auth.clone())?;
                let url = parse_server_url(&jmap_config.server)?;
                inner = inner.connect_jmap(&url, &tls, http_auth)?;
            }
        }

        #[cfg(feature = "gmail")]
        if backend.allows_gmail() {
            if let Some(gmail_config) = account_config.gmail.take() {
                use secrecy::ExposeSecret;

                use crate::gmail::client::gmail_token;

                let tls = gmail_config.tls.clone().into_tls(gmail_config.alpn.clone());
                let token = gmail_token(gmail_config.auth.clone())?;
                inner = inner.connect_gmail(
                    &tls,
                    token.expose_secret(),
                    gmail_config.user_id.clone(),
                )?;
            }
        }

        #[cfg(feature = "msgraph")]
        if backend.allows_msgraph() {
            if let Some(msgraph_config) = account_config.msgraph.take() {
                use secrecy::ExposeSecret;

                use crate::msgraph::client::msgraph_token;

                let tls = msgraph_config
                    .tls
                    .clone()
                    .into_tls(msgraph_config.alpn.clone());
                let token = msgraph_token(msgraph_config.auth.clone())?;
                inner = inner.connect_msgraph(
                    &tls,
                    token.expose_secret(),
                    msgraph_config.user_id.clone(),
                )?;
            }
        }

        #[cfg(feature = "imap")]
        if backend.allows_imap() {
            if let Some(imap_config) = account_config.imap.take() {
                use io_email::imap::client::ImapClientStd;
                use pimalaya_stream::sasl::Sasl;

                use crate::imap::id::resolve_auto_id_params;

                let tls = imap_config.tls.into_tls(imap_config.alpn);
                let auto_id = resolve_auto_id_params(&imap_config.id)?;
                let server = crate::imap::client::parse_imap_server(&imap_config.server)?;
                let sasl: Option<Sasl> = imap_config
                    .sasl
                    .and_then(|cfg| {
                        let host = server.host_str()?;
                        let port = server.port().unwrap_or(993);
                        Some(cfg.try_into_sasl(host, port))
                    })
                    .transpose()?;
                let imap =
                    ImapClientStd::connect(&server, &tls, imap_config.starttls, sasl, auto_id)?;
                inner = inner.with_imap(imap);
            }
        }

        #[cfg(feature = "maildir")]
        if backend.allows_maildir() {
            if let Some(maildir_config) = account_config.maildir.take() {
                use io_email::maildir::client::MaildirClient;

                let client = MaildirClient::new(maildir_config.root.to_string_lossy().into_owned());
                inner = inner.with_maildir(client);
            }
        }

        #[cfg(feature = "m2dir")]
        if backend.allows_m2dir() {
            if let Some(m2dir_config) = account_config.m2dir.take() {
                use io_email::m2dir::client::M2dirClient;

                let client = M2dirClient::new(m2dir_config.root.to_string_lossy().into_owned());
                inner = inner.with_m2dir(client);
            }
        }

        // Register SMTP alongside the storage backend so shared
        // `send_message` works for IMAP/Maildir accounts. JMAP already
        // sends via submission; the dispatch priority (JMAP → SMTP in
        // the send path) keeps that working when both are present.
        // SMTP also counts as a configured backend on its own, so
        // accounts with only `[smtp]` populated still construct and
        // can run `message send`. SMTP is initialized regardless of
        // the `--backend` flag so an explicit storage pin (e.g.
        // `--backend imap`) does not drop the send transport.
        #[cfg(feature = "smtp")]
        if backend.allows_smtp() {
            if let Some(smtp_config) = account_config.smtp.take() {
                use std::net::Ipv4Addr;

                use io_email::smtp::client::SmtpClientStd;
                use io_smtp::rfc5321::types::ehlo_domain::EhloDomain;
                use pimalaya_stream::sasl::Sasl;

                let tls = smtp_config.tls.into_tls(smtp_config.alpn);
                let domain: EhloDomain<'static> = Ipv4Addr::new(127, 0, 0, 1).into();
                let server = crate::smtp::client::parse_smtp_server(&smtp_config.server)?;
                let sasl: Option<Sasl> = smtp_config
                    .sasl
                    .and_then(|cfg| {
                        let host = server.host_str()?;
                        let port = server.port().unwrap_or(587);
                        Some(cfg.try_into_sasl(host, port))
                    })
                    .transpose()?;
                inner = inner.with_smtp(SmtpClientStd::connect(
                    &server,
                    &tls,
                    smtp_config.starttls,
                    domain,
                    sasl,
                )?);
            }
        }

        let account = Account::from(config).merge(Account::from(account_config));

        Ok((account, Self { inner }))
    }
}

impl Deref for EmailClient {
    type Target = EmailClientStd;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for EmailClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

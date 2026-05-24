// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Cross-protocol [`EmailClient`] for the shared subcommands
//! (`mailboxes`, `envelopes`, `flags`, `messages`, `attachments`).
//!
//! Wraps [`io_email::client::EmailClientStd`] and bundles the active
//! [`Account`] (display, identity, composer/reader registries) the
//! shared commands need alongside the I/O client. Implements
//! [`Deref`]/[`DerefMut`] onto the inner client so callers can call
//! its methods directly.
//!
//! Construction picks the first storage backend (`jmap → imap →
//! maildir`) allowed by the `BackendFlag` that is configured on the
//! account. When the account also has SMTP configured, an SMTP slot
//! is registered on the same client so `send_message` works for
//! IMAP/Maildir accounts; JMAP accounts already send via JMAP
//! submission. SMTP connection failures are logged and skipped — the
//! client still opens for reading.

use std::ops::{Deref, DerefMut};

use anyhow::{Result, bail};

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
    pub fn new(
        config: Config,
        mut account_config: AccountConfig,
        backend: Backend,
    ) -> Result<Self> {
        use io_email::client::EmailClientStd;

        let mut inner = EmailClientStd::new();
        let mut configured = false;

        #[cfg(feature = "jmap")]
        if !configured && backend.allows_jmap() {
            if let Some(jmap_config) = account_config.jmap.take() {
                use io_jmap::client::JmapClientStd;
                use pimalaya_stream::tls::Tls;

                use crate::jmap::client::{jmap_http_auth, parse_server_url};

                let mut tls: Tls = jmap_config.tls.clone().into();
                tls.rustls.alpn = vec!["http/1.1".into()];
                let http_auth = jmap_http_auth(jmap_config.auth.clone())?;
                let url = parse_server_url(&jmap_config.server)?;
                let mut client = JmapClientStd::connect(&url, &tls, http_auth)?;
                client.session_get(&url)?;

                inner = inner.with_jmap(client);
                configured = true;
            }
        }

        #[cfg(feature = "imap")]
        if !configured && backend.allows_imap() {
            if let Some(imap_config) = account_config.imap.take() {
                use io_imap::client::ImapClientStd;
                use io_imap::types::core::{IString, NString};
                use pimalaya_stream::{sasl::Sasl, std::stream::StreamStd, tls::Tls};

                let mut tls: Tls = imap_config.tls.into();
                tls.rustls.alpn = vec!["imap".into()];
                let sasl: Option<Sasl> = imap_config.sasl.map(Sasl::try_from).transpose()?;
                let server = crate::imap::client::parse_imap_server(&imap_config.server)?;
                let mut client =
                    ImapClientStd::<StreamStd>::connect(&server, &tls, imap_config.starttls, sasl)?;

                // Send IMAP ID (RFC 2971) after authentication when configured.
                // Some servers (e.g. NetEase 163) require this before allowing
                // mailbox SELECT.
                if imap_config.extensions.id.send_after_auth {
                    let id_cfg = &imap_config.extensions.id;
                    let name = id_cfg
                        .name
                        .clone()
                        .unwrap_or_else(|| "Himalaya".into());
                    let version = id_cfg
                        .version
                        .clone()
                        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").into());
                    let vendor = id_cfg
                        .vendor
                        .clone()
                        .unwrap_or_else(|| "Pimalaya".into());
                    let support_email = id_cfg
                        .support_email
                        .clone()
                        .unwrap_or_else(|| "pimalaya.org@posteo.net".into());

                    let id_fields: Vec<(IString<'static>, NString<'static>)> = [
                        ("name", NString::try_from(name.into_bytes()).unwrap()),
                        (
                            "version",
                            NString::try_from(version.into_bytes()).unwrap(),
                        ),
                        ("vendor", NString::try_from(vendor.into_bytes()).unwrap()),
                        (
                            "support-email",
                            NString::try_from(support_email.into_bytes()).unwrap(),
                        ),
                    ]
                    .into_iter()
                    .map(|(k, v)| (IString::try_from(k).unwrap(), v))
                    .collect();

                    if let Err(err) = client.id(Some(id_fields)) {
                        log::warn!("IMAP ID command failed (non-fatal): {err}");
                    }
                }

                inner = inner.with_imap(client);
                configured = true;
            }
        }

        #[cfg(feature = "maildir")]
        if !configured && backend.allows_maildir() {
            if let Some(maildir_config) = account_config.maildir.take() {
                use io_maildir::client::MaildirClient;

                let client = MaildirClient::new(maildir_config.root.to_string_lossy().into_owned());

                inner = inner.with_maildir(client);
                configured = true;
            }
        }

        #[cfg(feature = "m2dir")]
        if !configured && backend.allows_m2dir() {
            if let Some(m2dir_config) = account_config.m2dir.take() {
                use io_m2dir::client::M2dirClient;

                let client = M2dirClient::new(m2dir_config.root.to_string_lossy().into_owned());

                inner = inner.with_m2dir(client);
                configured = true;
            }
        }

        if !configured {
            bail!("no backend matching `{backend}` is configured for this account");
        }

        // Register SMTP alongside the storage backend so shared
        // `send_message` works for IMAP/Maildir accounts. JMAP already
        // sends via submission, but if both are present, SMTP wins
        // because storage is registered first.
        #[cfg(feature = "smtp")]
        if let Some(smtp_config) = account_config.smtp.take() {
            use std::net::Ipv4Addr;

            use io_smtp::{client::SmtpClientStd, rfc5321::types::ehlo_domain::EhloDomain};
            use pimalaya_stream::{sasl::Sasl, std::stream::StreamStd, tls::Tls};

            let smtp = (|| -> Result<SmtpClientStd<StreamStd>> {
                let mut tls: Tls = smtp_config.tls.into();
                tls.rustls.alpn = vec!["smtp".into()];
                let sasl: Option<Sasl> = smtp_config.sasl.map(Sasl::try_from).transpose()?;
                let domain: EhloDomain<'static> = Ipv4Addr::new(127, 0, 0, 1).into();
                let server = crate::smtp::client::parse_smtp_server(&smtp_config.server)?;
                Ok(SmtpClientStd::<StreamStd>::connect(
                    &server,
                    &tls,
                    smtp_config.starttls,
                    domain,
                    sasl,
                )?)
            })();

            match smtp {
                Ok(client) => inner = inner.with_smtp(client),
                Err(err) => {
                    log::warn!("SMTP backend disabled: {err}. Sending will be unavailable.")
                }
            }
        }

        let account = Account::from(config).merge(Account::from(account_config));

        Ok(Self { inner, account })
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

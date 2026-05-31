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

//! Himalaya wrapper around [`io_smtp::client::SmtpClientStd`] that
//! bundles the merged [`Account`] alongside the live SMTP client.
//!
//! Built up front by the dispatch layer (`crate::cli`) via
//! [`build_smtp_client`] and handed down to every SMTP-specific
//! subcommand. SMTP send is stateless after auth, so no session
//! context needs to follow the stream.

use std::{
    net::Ipv4Addr,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use io_smtp::{client::SmtpClientStd as Inner, rfc5321::types::ehlo_domain::EhloDomain};
use pimalaya_config::toml::TomlConfig;
use pimalaya_stream::{sasl::Sasl, tls::Tls};
use url::Url;

use crate::{account::context::Account, cli::load_or_wizard, config::SmtpConfig};

pub struct SmtpClient {
    inner: Inner,
    #[allow(dead_code)]
    pub account: Account,
}

impl SmtpClient {
    /// Opens the SMTP connection (TCP/TLS/STARTTLS, greeting, EHLO,
    /// SASL) then wraps the resulting client alongside `account`.
    pub fn new(config: SmtpConfig, account: Account) -> Result<Self> {
        let mut tls: Tls = config.tls.into();
        tls.rustls.alpn = vec!["smtp".into()];
        let sasl: Option<Sasl> = config.sasl.map(Sasl::try_from).transpose()?;
        let domain: EhloDomain<'static> = Ipv4Addr::new(127, 0, 0, 1).into();
        let server = parse_smtp_server(&config.server)?;
        let inner = Inner::connect(&server, &tls, config.starttls, domain, sasl)?;
        Ok(Self { inner, account })
    }
}

/// Parses an SMTP server string into a URL.
///
/// Accepts a bare authority (`smtp.example.com`, optionally with a
/// port), which is treated as `smtps://<authority>` (secure by
/// default); or a full URL whose scheme (`smtp` or `smtps`) is used
/// verbatim. Mirrors the JMAP server-string handling.
pub fn parse_smtp_server(server: &str) -> Result<Url> {
    match Url::parse(server) {
        Ok(url) => Ok(url),
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            Ok(Url::parse(&format!("smtps://{server}"))?)
        }
        Err(err) => Err(err.into()),
    }
}

impl Deref for SmtpClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for SmtpClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Loads the configuration, picks the active account, builds the
/// merged [`Account`] then opens the SMTP session. Bails when the
/// account has no `[smtp]` block.
pub fn build_smtp_client(
    config_paths: &[PathBuf],
    account_name: Option<&str>,
) -> Result<SmtpClient> {
    let mut config = load_or_wizard(config_paths)?;
    let (name, mut ac) = config
        .take_account(account_name)?
        .ok_or_else(|| anyhow!("Cannot find account"))?;
    let smtp_config = ac
        .smtp
        .take()
        .ok_or_else(|| anyhow!("SMTP config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(ac));
    SmtpClient::new(smtp_config, account)
}

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

//! Himalaya wrapper around [`io_imap::client::ImapClientStd`].
//!
//! This is what every IMAP-specific subcommand receives: the dispatch
//! layer (`crate::cli`) opens the session up front via
//! [`build_imap_client`] and hands the ready-to-use wrapper down,
//! together with the merged [`Account`] as a sibling argument.

use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use io_imap::client::ImapClientStd as Inner;
use pimalaya_config::toml::TomlConfig;
use pimalaya_stream::{sasl::Sasl, tls::Tls};
use url::Url;

use crate::{
    account::context::Account, cli::load_or_wizard, config::ImapConfig,
    imap::id::resolve_auto_id_params,
};

pub struct ImapClient {
    inner: Inner,
}

impl ImapClient {
    /// Opens the IMAP connection (TCP/TLS/STARTTLS, greeting, SASL).
    /// The capability list reported by the connect handshake is
    /// discarded; IMAP-specific subcommands that need it should call
    /// [`Inner::capability`] explicitly.
    pub fn new(config: ImapConfig) -> Result<Self> {
        let mut tls: Tls = config.tls.into();
        tls.rustls.alpn = vec!["imap".into()];
        let sasl: Option<Sasl> = config.sasl.map(Sasl::try_from).transpose()?;
        let auto_id = resolve_auto_id_params(&config.id)?;
        let server = parse_imap_server(&config.server)?;
        let (inner, _capability) = Inner::connect(&server, &tls, config.starttls, sasl, auto_id)?;
        Ok(Self { inner })
    }
}

/// Parses an IMAP server string into a URL.
///
/// Accepts a bare authority (`imap.example.com`, optionally with a
/// port), which is treated as `imaps://<authority>` (secure by
/// default); or a full URL whose scheme (`imap` or `imaps`) is used
/// verbatim. Mirrors the JMAP server-string handling.
pub fn parse_imap_server(server: &str) -> Result<Url> {
    match Url::parse(server) {
        Ok(url) => Ok(url),
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            Ok(Url::parse(&format!("imaps://{server}"))?)
        }
        Err(err) => Err(err.into()),
    }
}

impl Deref for ImapClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ImapClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Loads the configuration, picks the active account, builds the
/// merged [`Account`] then opens the IMAP session. Bails when the
/// account has no `[imap]` block. Returns the live client paired
/// with the merged account so subcommands receive both as sibling
/// arguments.
pub fn build_imap_client(
    config_paths: &[PathBuf],
    account_name: Option<&str>,
) -> Result<(Account, ImapClient)> {
    let mut config = load_or_wizard(config_paths)?;
    let (name, mut ac) = config
        .take_account(account_name)?
        .ok_or_else(|| anyhow!("Cannot find account"))?;
    let imap_config = ac
        .imap
        .take()
        .ok_or_else(|| anyhow!("IMAP config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(ac));
    let client = ImapClient::new(imap_config)?;
    Ok((account, client))
}

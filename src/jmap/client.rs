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

//! Himalaya wrapper around [`io_jmap::client::JmapClientStd`] that
//! bundles the merged [`Account`] alongside the live JMAP client.
//!
//! Built up front by the dispatch layer (`crate::cli`) via
//! [`build_jmap_client`] and handed down to every JMAP-specific
//! subcommand.

use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use base64::{prelude::BASE64_STANDARD, Engine};
use io_jmap::client::JmapClientStd as Inner;
use pimalaya_config::toml::TomlConfig;
use pimalaya_stream::tls::Tls;
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::{
    account::context::Account,
    cli::load_or_wizard,
    config::{JmapAuthConfig, JmapConfig},
};

pub struct JmapClient {
    inner: Inner,
    pub account: Account,
    /// The original JMAP config block, kept around so commands like
    /// `email import` / `email export` can spin up their own
    /// auxiliary sessions (e.g. against the upload/download URL when
    /// it lives on a different authority than the API URL).
    pub config: JmapConfig,
}

impl JmapClient {
    /// Establishes the JMAP session (TLS, `/.well-known/jmap`
    /// discovery) then wraps the resulting client alongside
    /// `account`.
    pub fn new(config: JmapConfig, account: Account) -> Result<Self> {
        let mut tls: Tls = config.tls.clone().into();
        tls.rustls.alpn = vec!["http/1.1".into()];

        let http_auth = jmap_http_auth(config.auth.clone())?;
        let url = parse_server_url(&config.server)?;

        let mut inner = Inner::connect(&url, &tls, http_auth)?;
        inner.session_get(&url)?;

        Ok(Self {
            inner,
            account,
            config,
        })
    }
}

impl Deref for JmapClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for JmapClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Loads the configuration, picks the active account, builds the
/// merged [`Account`] then opens the JMAP session. Bails when the
/// account has no `[jmap]` block.
pub fn build_jmap_client(
    config_paths: &[PathBuf],
    account_name: Option<&str>,
) -> Result<JmapClient> {
    let mut config = load_or_wizard(config_paths)?;
    let (name, mut ac) = config
        .take_account(account_name)?
        .ok_or_else(|| anyhow!("Cannot find account"))?;
    let jmap_config = ac
        .jmap
        .take()
        .ok_or_else(|| anyhow!("JMAP config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(ac));
    JmapClient::new(jmap_config, account)
}

/// Parses the JMAP `server` field into a [`Url`], defaulting bare
/// authorities (e.g. `mail.example.com`) to `https://`.
pub fn parse_server_url(server: &str) -> Result<Url> {
    match Url::parse(server) {
        Ok(url) => Ok(url),
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            Ok(Url::parse(&format!("https://{server}"))?)
        }
        Err(err) => Err(err.into()),
    }
}

/// Converts a [`JmapAuthConfig`] into the pre-formatted HTTP
/// `Authorization` header value [`JmapClientStd::connect`] expects.
///
/// [`JmapClientStd::connect`]: io_jmap::client::JmapClientStd::connect
pub fn jmap_http_auth(config: JmapAuthConfig) -> Result<SecretString> {
    match config {
        JmapAuthConfig::Header(token) => Ok(token.get()?),
        JmapAuthConfig::Bearer { token } => {
            let token = token.get()?;
            Ok(format!("Bearer {}", token.expose_secret()).into())
        }
        JmapAuthConfig::Basic { username, password } => {
            let creds = format!("{}:{}", username, password.get()?.expose_secret());
            let encoded = BASE64_STANDARD.encode(creds.into_bytes());
            Ok(format!("Basic {encoded}").into())
        }
    }
}

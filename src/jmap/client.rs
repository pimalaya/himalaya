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

use anyhow::{Result, anyhow};
use base64::{Engine, prelude::BASE64_STANDARD};
use io_jmap::client::JmapClientStd as Inner;
use pimalaya_config::toml::TomlConfig;
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::{
    account::context::Account,
    cli::load_or_wizard,
    config::{JmapAuthConfig, JmapConfig, parse_server},
};

/// Live JMAP session paired with the merged account configuration.
pub struct JmapClient {
    inner: Inner,
    /// The original JMAP config block, kept around so commands like
    /// `email import` / `email export` can spin up their own
    /// auxiliary sessions (e.g. against the upload/download URL when
    /// it lives on a different authority than the API URL).
    pub config: JmapConfig,
}

impl JmapClient {
    /// Establishes the JMAP session (TLS, `/.well-known/jmap`
    /// discovery).
    pub fn new(config: JmapConfig) -> Result<Self> {
        let tls = config.tls.clone().into_tls(config.alpn.clone());

        let http_auth = jmap_http_auth(config.auth.clone())?;
        let url = parse_server_url(&config.server)?;

        let mut inner = Inner::connect(&url, &tls, http_auth)?;
        inner.session_get(&url)?;

        Ok(Self { inner, config })
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
/// account has no `[jmap]` block. Returns the live client paired
/// with the merged account so subcommands receive both as sibling
/// arguments.
pub fn build_jmap_client(
    config_paths: &[PathBuf],
    account_name: Option<&str>,
) -> Result<(Account, JmapClient)> {
    let mut config = load_or_wizard(config_paths)?;
    let (name, mut ac) = config
        .take_account(account_name)?
        .ok_or_else(|| anyhow!("Cannot find account"))?;
    let jmap_config = ac
        .jmap
        .take()
        .ok_or_else(|| anyhow!("JMAP config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(ac));
    let client = JmapClient::new(jmap_config)?;
    Ok((account, client))
}

/// Parses the JMAP `server` field into a [`Url`]. Accepts a full
/// `http`/`https://host[:port][/path]` URL, a bare `host:port`, or a
/// bare `host`; the last two default to `https://` (secure). Any other
/// scheme is rejected.
pub fn parse_server_url(server: &str) -> Result<Url> {
    parse_server(server, "https", &["http", "https"])
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

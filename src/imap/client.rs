//! Himalaya wrapper around [`io_imap::client::ImapClientStd`] that
//! bundles the merged [`Account`] alongside the live IMAP client.
//!
//! This is what every IMAP-specific subcommand receives: the dispatch
//! layer (`crate::cli`) opens the session up front via
//! [`build_imap_client`] and hands the ready-to-use wrapper down.

use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use io_imap::client::ImapClientStd as Inner;
use pimalaya_config::toml::TomlConfig;
use pimalaya_stream::{sasl::Sasl, std::stream::StreamStd, tls::Tls};
use url::Url;

use crate::{account::context::Account, cli::load_or_wizard, config::ImapConfig};

pub struct ImapClient {
    inner: Inner<StreamStd>,
    pub account: Account,
}

impl ImapClient {
    /// Opens the IMAP connection (TCP/TLS/STARTTLS, greeting, SASL)
    /// then wraps the resulting client alongside `account`.
    pub fn new(config: ImapConfig, account: Account) -> Result<Self> {
        let mut tls: Tls = config.tls.into();
        tls.rustls.alpn = vec!["imap".into()];
        let sasl: Option<Sasl> = config.sasl.map(Sasl::try_from).transpose()?;
        let server = parse_imap_server(&config.server)?;
        let inner = Inner::<StreamStd>::connect(&server, &tls, config.starttls, sasl)?;
        Ok(Self { inner, account })
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
    type Target = Inner<StreamStd>;

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
/// account has no `[imap]` block.
pub fn build_imap_client(
    config_paths: &[PathBuf],
    account_name: Option<&str>,
) -> Result<ImapClient> {
    let mut config = load_or_wizard(config_paths)?;
    let (name, mut ac) = config
        .take_account(account_name)?
        .ok_or_else(|| anyhow!("Cannot find account"))?;
    let imap_config = ac
        .imap
        .take()
        .ok_or_else(|| anyhow!("IMAP config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(ac));
    ImapClient::new(imap_config, account)
}

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
use pimalaya_stream::sasl::Sasl;
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
        let tls = config.tls.into_tls(config.alpn);
        let auto_id = resolve_auto_id_params(&config.id)?;
        let server = parse_imap_server(&config.server)?;
        let sasl: Option<Sasl> = config
            .sasl
            .and_then(|cfg| {
                let host = server.host_str()?;
                let port = server.port_or_known_default()?;
                Some(cfg.try_into_sasl(host, port))
            })
            .transpose()?;
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

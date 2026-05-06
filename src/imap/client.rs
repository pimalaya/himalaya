//! Himalaya wrapper around [`io_imap::client::ImapClient`] that
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
use io_imap::client::ImapClient as Inner;
use pimalaya_config::toml::TomlConfig;

use crate::{
    account::context::Account, cli::load_or_wizard, config::ImapConfig, imap::session::ImapSession,
};

pub struct ImapClient {
    inner: Inner,
    pub account: Account,
}

impl ImapClient {
    /// Opens the IMAP connection (TCP/TLS/STARTTLS, greeting, SASL)
    /// then wraps the resulting stream + context in an
    /// [`io_imap::client::ImapClient`] alongside `account`.
    pub fn new(config: ImapConfig, account: Account) -> Result<Self> {
        let session = ImapSession::new(
            config.url,
            config.tls.try_into()?,
            config.starttls,
            config.sasl.try_into()?,
        )?;
        let inner = Inner::from_parts(session.stream, session.context);
        Ok(Self { inner, account })
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

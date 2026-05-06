//! Himalaya wrapper around [`io_smtp::client::SmtpClient`] that
//! bundles the merged [`Account`] alongside the live SMTP client.
//!
//! Built up front by the dispatch layer (`crate::cli`) via
//! [`build_smtp_client`] and handed down to every SMTP-specific
//! subcommand. SMTP send is stateless after auth, so no session
//! context needs to follow the stream.

use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use io_smtp::client::SmtpClient as Inner;
use pimalaya_config::toml::TomlConfig;

use crate::{
    account::context::Account, cli::load_or_wizard, config::SmtpConfig, smtp::session::SmtpSession,
};

pub struct SmtpClient {
    inner: Inner,
    #[allow(dead_code)]
    pub account: Account,
}

impl SmtpClient {
    /// Opens the SMTP connection (TCP/TLS/STARTTLS, greeting, EHLO,
    /// SASL) then wraps the resulting stream alongside `account`.
    pub fn new(config: SmtpConfig, account: Account) -> Result<Self> {
        let session = SmtpSession::new(
            config.url,
            config.tls.try_into()?,
            config.starttls,
            config.sasl.try_into()?,
        )?;
        let inner = Inner::new(session.stream);
        Ok(Self { inner, account })
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

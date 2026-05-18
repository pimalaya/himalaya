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

use anyhow::{anyhow, Result};
use io_smtp::{client::SmtpClientStd as Inner, rfc5321::types::ehlo_domain::EhloDomain};
use pimalaya_config::toml::TomlConfig;
use pimalaya_stream::{sasl::Sasl, std::stream::StreamStd, tls::Tls};

use crate::{account::context::Account, cli::load_or_wizard, config::SmtpConfig};

pub struct SmtpClient {
    inner: Inner<StreamStd>,
    #[allow(dead_code)]
    pub account: Account,
}

impl SmtpClient {
    /// Opens the SMTP connection (TCP/TLS/STARTTLS, greeting, EHLO,
    /// SASL) then wraps the resulting client alongside `account`.
    pub fn new(config: SmtpConfig, account: Account) -> Result<Self> {
        let mut tls: Tls = config.tls.into();
        tls.rustls.alpn = vec!["smtp".into()];
        let sasl: Sasl = config.sasl.try_into()?;
        let domain: EhloDomain<'static> = Ipv4Addr::new(127, 0, 0, 1).into();
        let inner =
            Inner::<StreamStd>::connect(&config.url, &tls, config.starttls, domain, Some(sasl))?;
        Ok(Self { inner, account })
    }
}

impl Deref for SmtpClient {
    type Target = Inner<StreamStd>;

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

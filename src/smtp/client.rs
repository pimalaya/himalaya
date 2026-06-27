//! Himalaya wrapper around [`io_smtp::client::SmtpClientStd`].
//!
//! Built up front by the dispatch layer (`crate::cli`) via
//! [`build_smtp_client`] and handed down to every SMTP-specific
//! subcommand, together with the merged [`Account`] as a sibling
//! argument. SMTP send is stateless after auth, so no session
//! context needs to follow the stream.

use std::{
    net::Ipv4Addr,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use io_smtp::{client::SmtpClientStd as Inner, rfc5321::types::ehlo_domain::EhloDomain};
use pimalaya_config::toml::TomlConfig;
use pimalaya_stream::sasl::Sasl;
use url::Url;

use crate::{
    account::context::Account,
    cli::load_or_wizard,
    config::{SmtpConfig, parse_server},
};

pub struct SmtpClient {
    inner: Inner,
}

impl SmtpClient {
    /// Opens the SMTP connection (TCP/TLS/STARTTLS, greeting, EHLO,
    /// SASL).
    pub fn new(config: SmtpConfig) -> Result<Self> {
        let tls = config.tls.into_tls(config.alpn);
        let domain: EhloDomain<'static> = Ipv4Addr::new(127, 0, 0, 1).into();
        let server = parse_smtp_server(&config.server)?;
        let sasl: Option<Sasl> = config
            .sasl
            .and_then(|cfg| {
                let host = server.host_str()?;
                let port = server.port_or_known_default()?;
                Some(cfg.try_into_sasl(host, port))
            })
            .transpose()?;
        let inner = Inner::connect(&server, &tls, config.starttls, domain, sasl)?;
        Ok(Self { inner })
    }
}

/// Parses an SMTP server string into a URL.
///
/// Accepts `smtp`/`smtps://host[:port]`, a bare `host:port`, or a bare
/// `host`; the last two default to `smtps://` (secure). Any other
/// scheme is rejected.
pub fn parse_smtp_server(server: &str) -> Result<Url> {
    parse_server(server, "smtps", &["smtp", "smtps"])
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
/// account has no `[smtp]` block. Returns the live client paired
/// with the merged account so subcommands receive both as sibling
/// arguments.
pub fn build_smtp_client(
    config_paths: &[PathBuf],
    account_name: Option<&str>,
) -> Result<(Account, SmtpClient)> {
    let mut config = load_or_wizard(config_paths)?;
    let (name, mut ac) = config
        .take_account(account_name)?
        .ok_or_else(|| anyhow!("Cannot find account"))?;
    let smtp_config = ac
        .smtp
        .take()
        .ok_or_else(|| anyhow!("SMTP config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(ac));
    let client = SmtpClient::new(smtp_config)?;
    Ok((account, client))
}

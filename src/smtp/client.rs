//! Himalaya wrapper around [`io_smtp::client::SmtpClientStd`].
//!
//! Built up front by the dispatch layer (`crate::cli`) via
//! [`build_smtp_client`] and handed down to every SMTP-specific
//! subcommand. SMTP send is stateless after auth, so unlike the
//! storage backends the commands need no account context: the merged
//! [`Account`] returned by [`build_smtp_client`] (for dispatch
//! uniformity) is not threaded into them.

use std::{
    net::Ipv4Addr,
    ops::{Deref, DerefMut},
};

use anyhow::{Result, anyhow};
use io_sasl::mechanism::Sasl;
use io_smtp::{
    client::SmtpClientStd as Inner, rfc5321::SmtpEhloDomain, session::SmtpSessionOpenOptions,
};
use url::Url;

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, SmtpConfig, parse_server},
};

/// SMTP client wrapping the inner stream for sending messages.
pub struct SmtpClient {
    inner: Inner,
}

impl SmtpClient {
    /// Opens the SMTP connection (TCP/TLS/STARTTLS, greeting, EHLO,
    /// SASL).
    pub fn new(config: SmtpConfig) -> Result<Self> {
        let tls = config.tls.into_tls(config.alpn);
        let domain: SmtpEhloDomain<'static> = Ipv4Addr::new(127, 0, 0, 1).into();
        let server = parse_smtp_server(&config.server)?;
        let sasl: Option<Sasl> = match config.sasl {
            // A `unix://` sirup socket presents a pre-authenticated
            // session, so no SASL is negotiated over it.
            Some(_) if server.scheme() == "unix" => None,
            Some(cfg) => {
                let host = server
                    .host_str()
                    .ok_or_else(|| anyhow!("Cannot derive host from SMTP server `{server}`"))?;
                // url does not know the smtp(s) default ports; match
                // io-smtp's own connection defaults (465 for smtps).
                let port = server
                    .port()
                    .unwrap_or(Inner::default_port(server.scheme()));
                Some(cfg.try_into_sasl(host, port)?)
            }
            None => None,
        };
        let opts = SmtpSessionOpenOptions {
            starttls: config.starttls,
        };
        let (inner, _capabilities) = Inner::connect(&server, &tls, domain, sasl, opts)?;
        Ok(Self { inner })
    }
}

/// Parses an SMTP server string into a URL.
///
/// Accepts `smtp`/`smtps://host[:port]`, a bare `host:port`, or a bare
/// `host` (the last two default to `smtps://`, secure), or a
/// `unix:///path` socket for a local proxy such as sirup. Any other
/// scheme is rejected.
pub fn parse_smtp_server(server: &str) -> Result<Url> {
    parse_server(server, "smtps", &["smtp", "smtps", "unix"])
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

/// Opens the SMTP session for an already-resolved account: takes the
/// `[smtp]` block out of `account_config` and builds the merged
/// [`Account`]. Bails when the account has no `[smtp]` block. Returns the
/// live client paired with the merged account for dispatch uniformity
/// with the other `build_*_client` helpers, though SMTP subcommands
/// ignore the account.
pub fn build_smtp_client(
    config: Config,
    name: String,
    mut account_config: AccountConfig,
) -> Result<(Account, SmtpClient)> {
    let smtp_config = account_config
        .smtp
        .take()
        .ok_or_else(|| anyhow!("SMTP config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(account_config));
    let client = SmtpClient::new(smtp_config)?;
    Ok((account, client))
}

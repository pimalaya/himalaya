//! # ManageSieve client
//!
//! The wrapper around io-managesieve's blocking client every `sieve`
//! subcommand receives.
//!
//! The library opens the whole session, so this layer only turns the
//! `[sieve]` block into its arguments.

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use io_managesieve::{
    client::ManagesieveClientStd as Inner, session::ManagesieveSessionOpenOptions,
};
use io_sasl::mechanism::Sasl;
use url::Url;

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, SieveConfig, parse_server},
};

/// ManageSieve client wrapping the inner stream for script management.
pub struct SieveClient {
    inner: Inner,
}

impl SieveClient {
    /// Opens the ManageSieve session (TCP/TLS/STARTTLS, greeting,
    /// SASL).
    pub fn new(config: SieveConfig) -> Result<Self> {
        let tls = config.tls.into_tls(config.alpn);
        let server = parse_sieve_server(&config.server)?;
        let sasl: Option<Sasl> = match config.sasl {
            // NOTE: a `unix://` socket is already authenticated, so no
            // SASL is negotiated over it.
            Some(_) if server.scheme() == "unix" => None,
            Some(cfg) => {
                let host = server.host_str().ok_or_else(|| {
                    anyhow!("Cannot derive host from ManageSieve server `{server}`")
                })?;
                // NOTE: url knows no sieve default port, so the fallback is
                // io-managesieve's own, 4190 either way.
                let port = server
                    .port()
                    .unwrap_or(Inner::default_port(server.scheme()));
                Some(cfg.try_into_sasl(host, port)?)
            }
            None => None,
        };
        // NOTE: RFC 5804 registers one port and reaches TLS on it through
        // STARTTLS, so a `sieve://` server wants the upgrade by default.
        let opts = ManagesieveSessionOpenOptions {
            starttls: config.starttls.unwrap_or(server.scheme() == "sieve"),
            allow_cleartext_auth: config.allow_cleartext_auth,
        };
        let (inner, _capabilities) = Inner::connect(&server, &tls, sasl, opts)?;
        Ok(Self { inner })
    }
}

/// Parses a ManageSieve server string into a URL.
///
/// Accepts `sieve`/`sieves://host[:port]`, a bare `host:port`, or a
/// bare `host` (the last two default to `sieve://`), or a
/// `unix:///path` socket for a local proxy. Any other scheme is
/// rejected.
///
/// The bare default differs from the IMAP and SMTP ones, which resolve
/// to their implicit-TLS scheme: ManageSieve registers one port and no
/// implicit-TLS twin.
pub fn parse_sieve_server(server: &str) -> Result<Url> {
    parse_server(server, "sieve", &["sieve", "sieves", "unix"])
}

impl Deref for SieveClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for SieveClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Opens the ManageSieve session for an already-resolved account: takes
/// the `[sieve]` block out of `account_config` and builds the merged
/// [`Account`]. Bails when the account has no `[sieve]` block.
pub fn build_sieve_client(
    config: Config,
    name: String,
    mut account_config: AccountConfig,
) -> Result<(Account, SieveClient)> {
    let sieve_config = account_config
        .sieve
        .take()
        .ok_or_else(|| anyhow!("Sieve config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(account_config));
    let client = SieveClient::new(sieve_config)?;
    Ok((account, client))
}

#[cfg(test)]
mod tests {
    use crate::sieve::client::parse_sieve_server;

    #[test]
    fn a_bare_authority_resolves_to_starttls_rather_than_implicit_tls() {
        // NOTE: the one place the bare default differs from IMAP and SMTP,
        // ManageSieve registering no implicit-TLS port to reach.
        let url = parse_sieve_server("sieve.example.com").unwrap();
        assert_eq!(url.scheme(), "sieve");
        assert_eq!(url.port(), None);

        let url = parse_sieve_server("sieves://sieve.example.com:4190").unwrap();
        assert_eq!(url.scheme(), "sieves");
        assert_eq!(url.port(), Some(4190));

        let url = parse_sieve_server("unix:///run/sieve.sock").unwrap();
        assert_eq!(url.scheme(), "unix");

        assert!(parse_sieve_server("imaps://sieve.example.com").is_err());
    }
}

//! Himalaya wrapper around [`io_imap::client::ImapClientStd`].
//!
//! This is what every IMAP-specific subcommand receives: the dispatch
//! layer (`crate::cli`) opens the session up front via
//! [`build_imap_client`] and hands the ready-to-use wrapper down,
//! together with the merged [`Account`] as a sibling argument.

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use io_imap::{client::ImapClientStd as Inner, has_imap_capability, types::response::Capability};
use pimalaya_stream::sasl::Sasl;
use url::Url;

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, ImapConfig, parse_server},
    imap::id::resolve_auto_id_params,
};

/// Live IMAP client wrapping the io-imap session with cached
/// capabilities and the sort fallback policy.
pub struct ImapClient {
    inner: Inner,
    capabilities: Vec<Capability<'static>>,
    sort_fallback: Option<bool>,
}

impl ImapClient {
    /// Opens the IMAP connection (TCP/TLS/STARTTLS, greeting, SASL),
    /// caching the capability list reported by the handshake and the
    /// `imap.sort.fallback` config override for later policy checks.
    pub fn new(config: ImapConfig) -> Result<Self> {
        let sort_fallback = config.sort.fallback;
        let tls = config.tls.into_tls(config.alpn);
        let auto_id = resolve_auto_id_params(&config.id)?;
        let server = parse_imap_server(&config.server)?;
        let sasl: Option<Sasl> = match config.sasl {
            // A `unix://` sirup socket presents a pre-authenticated
            // session (the greeting is PREAUTH), so no SASL is negotiated.
            Some(_) if server.scheme() == "unix" => None,
            Some(cfg) => {
                let host = server
                    .host_str()
                    .ok_or_else(|| anyhow!("Cannot derive host from IMAP server `{server}`"))?;
                // url does not know the imap(s) default ports, so fall
                // back to the same scheme defaults io-imap connects with.
                let port = server
                    .port()
                    .unwrap_or(io_imap::client::default_port(server.scheme()));
                Some(cfg.try_into_sasl(host, port)?)
            }
            None => None,
        };
        let (inner, capabilities) = Inner::connect(&server, &tls, config.starttls, sasl, auto_id)?;
        Ok(Self {
            inner,
            capabilities,
            sort_fallback,
        })
    }

    /// Resolves the SORT fallback policy: the `imap.sort.fallback`
    /// config override when set, otherwise on only when the server
    /// lacks the SORT capability. When `true`, sort client-side via
    /// SEARCH + FETCH instead of issuing a server `SORT`.
    pub fn sort_fallback(&self) -> bool {
        self.sort_fallback
            .unwrap_or_else(|| !has_imap_capability!(self.capabilities, Sort(_)))
    }

    /// Whether the server advertised UIDPLUS (RFC 4315). When it does,
    /// `COPY`/`MOVE` return an authoritative `COPYUID` and an *absent*
    /// one means nothing was affected (a stale-UID no-op), so the count
    /// can be trusted; without it there is no such feedback.
    pub fn supports_uidplus(&self) -> bool {
        has_imap_capability!(self.capabilities, UidPlus)
    }
}

/// Parses an IMAP server string into a URL.
///
/// Accepts `imap`/`imaps://host[:port]`, a bare `host:port`, or a bare
/// `host` (the last two default to `imaps://`, secure), or a
/// `unix:///path` socket for a local proxy such as sirup. Any other
/// scheme is rejected.
pub fn parse_imap_server(server: &str) -> Result<Url> {
    parse_server(server, "imaps", &["imap", "imaps", "unix"])
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

/// Opens the IMAP session for an already-resolved account: takes the
/// `[imap]` block out of `account_config`, builds the merged [`Account`]
/// and connects. Bails when the account has no `[imap]` block. Returns
/// the live client paired with the merged account so subcommands receive
/// both as sibling arguments.
pub fn build_imap_client(
    config: Config,
    name: String,
    mut account_config: AccountConfig,
) -> Result<(Account, ImapClient)> {
    let imap_config = account_config
        .imap
        .take()
        .ok_or_else(|| anyhow!("IMAP config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(account_config));
    let client = ImapClient::new(imap_config)?;
    Ok((account, client))
}

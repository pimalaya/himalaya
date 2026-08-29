//! # IMAP client
//!
//! The wrapper around io-imap's blocking client every IMAP-specific
//! subcommand receives.
//!
//! The dispatch layer opens the session up front and hands the ready
//! wrapper down, the merged [`Account`] riding along as a sibling
//! argument.

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use io_imap::{
    client::{ImapClientStd as Inner, default_port},
    has_imap_capability,
    session::ImapSessionOpenOptions,
    types::response::Capability,
};
use io_sasl::mechanism::Sasl;
use url::Url;

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, ImapConfig, parse_server},
    imap::id::resolve_auto_id_params,
};

/// A live IMAP session, its capabilities and the sort fallback policy.
pub struct ImapClient {
    inner: Inner,
    capabilities: Vec<Capability<'static>>,
    sort_fallback: Option<bool>,
}

impl ImapClient {
    /// Opens the connection and authenticates, caching the capabilities
    /// the handshake reported.
    pub fn new(config: ImapConfig) -> Result<Self> {
        let sort_fallback = config.sort.fallback;
        let tls = config.tls.into_tls(config.alpn);
        let auto_id = resolve_auto_id_params(&config.id)?;
        let server = parse_imap_server(&config.server)?;
        let sasl: Option<Sasl> = match config.sasl {
            // NOTE: a `unix://` sirup socket greets with PREAUTH, so the
            // session is already authenticated and no SASL is negotiated.
            Some(_) if server.scheme() == "unix" => None,
            Some(cfg) => {
                let host = server
                    .host_str()
                    .ok_or_else(|| anyhow!("Cannot derive host from IMAP server `{server}`"))?;
                // NOTE: url knows no imap default port, so the fallback is
                // the same scheme default io-imap connects with.
                let port = server.port().unwrap_or(default_port(server.scheme()));
                Some(cfg.try_into_sasl(host, port)?)
            }
            None => None,
        };
        let opts = ImapSessionOpenOptions {
            starttls: config.starttls,
            auto_id,
            sasl_ir: config.sasl_ir,
        };
        let (inner, capabilities) = Inner::connect(&server, &tls, sasl, opts)?;
        Ok(Self {
            inner,
            capabilities,
            sort_fallback,
        })
    }

    /// Whether to sort client-side with SEARCH and FETCH rather than
    /// issue a server `SORT`.
    ///
    /// `imap.sort.fallback` decides when it is set, and the absence of
    /// the SORT capability otherwise.
    pub fn sort_fallback(&self) -> bool {
        self.sort_fallback
            .unwrap_or_else(|| !has_imap_capability!(self.capabilities, Sort(_)))
    }

    /// Whether the server advertised RFC 4315 UIDPLUS.
    ///
    /// With it, `COPY` and `MOVE` return an authoritative `COPYUID` whose
    /// absence means nothing was affected, so a count can be trusted.
    /// Without it there is no such feedback.
    pub fn supports_uidplus(&self) -> bool {
        has_imap_capability!(self.capabilities, UidPlus)
    }
}

/// Parses an IMAP server string into a URL.
///
/// A full `imap://` or `imaps://` URL, a bare authority or host taking
/// `imaps://`, or a `unix://` socket for a local proxy such as sirup.
/// Any other scheme is rejected.
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

/// Opens the IMAP session of an already-resolved account, returning it
/// beside the merged [`Account`].
///
/// Bails when the account declares no `[imap]` block.
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

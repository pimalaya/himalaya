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
    client::{ImapClientStd as Inner, ImapClientStdConnectOptions, default_port},
    has_imap_capability,
    session::ImapSessionOpenOptions,
    types::{extensions::thread::ThreadingAlgorithm, response::Capability},
};
use io_sasl::mechanism::Sasl;
use pimalaya_config::secret::SecretResolver;
use url::Url;

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, imap::ImapConfig, proxy::ProxyConfig},
    imap::id::resolve_auto_id_params,
    server::parse_server,
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
    pub fn new(config: ImapConfig, resolver: &mut SecretResolver) -> Result<Self> {
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
                Some(cfg.try_into_sasl(host, port, resolver)?)
            }
            None => None,
        };
        let opts = ImapClientStdConnectOptions {
            tls,
            proxy: ProxyConfig::resolve(config.proxy, resolver)?,
            sasl,
            session: ImapSessionOpenOptions {
                starttls: config.starttls,
                auto_id,
                sasl_ir: config.sasl_ir,
                ..Default::default()
            },
        };
        let (inner, capabilities) = Inner::connect(&server, opts)?;
        Ok(Self {
            inner,
            capabilities,
            sort_fallback,
        })
    }

    /// Whether to sort client-side with SEARCH and FETCH rather than
    /// issue a server `SORT`.
    ///
    /// `imap.sort.fallback` decides when it is set, and otherwise
    /// [`default_sort_fallback`] over the advertised capabilities.
    pub fn sort_fallback(&self) -> bool {
        self.sort_fallback
            .unwrap_or_else(|| default_sort_fallback(&self.capabilities))
    }

    /// Whether the server advertised RFC 4315 UIDPLUS.
    ///
    /// With it, `UID EXPUNGE` removes only the given messages. Its
    /// `COPYUID` is not relied on alone: a server sends none when nothing
    /// matched, and some send one that cannot be read.
    pub fn supports_uidplus(&self) -> bool {
        has_imap_capability!(self.capabilities, UidPlus)
    }

    /// Whether the server advertised RFC 6851 MOVE.
    pub fn supports_move(&self) -> bool {
        has_imap_capability!(self.capabilities, Move)
    }

    /// Whether the server advertised RFC 5256 THREAD with `algorithm`.
    pub fn supports_thread(&self, algorithm: &ThreadingAlgorithm<'_>) -> bool {
        self.capabilities.iter().any(|capability| {
            matches!(capability, Capability::Thread(advertised) if advertised == algorithm)
        })
    }
}

/// Whether to sort client-side when `imap.sort.fallback` is unset: the
/// server lacks SORT, or it is iCloud, whose SORT ignores its keys and
/// `REVERSE`.
///
/// iCloud is told by its `X-APPLE-REMOTE-LINKS` capability, which holds
/// behind a proxy and on every `*.mail.me.com` host.
fn default_sort_fallback(capabilities: &[Capability<'_>]) -> bool {
    let icloud = capabilities.iter().any(|capability| {
        matches!(capability, Capability::Other(_))
            && capability
                .to_string()
                .eq_ignore_ascii_case("X-APPLE-REMOTE-LINKS")
    });

    icloud || !has_imap_capability!(capabilities, Sort(_))
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
    let account = Account {
        name: Some(name.to_string()),
        ..Account::from(config).merge(Account::from(account_config))
    };
    let client = ImapClient::new(imap_config, &mut SecretResolver::new())?;
    Ok((account, client))
}

#[cfg(test)]
mod tests {
    use io_imap::types::response::Capability;

    use crate::imap::client::default_sort_fallback;

    fn capabilities(line: &str) -> Vec<Capability<'static>> {
        line.split(' ')
            .map(|atom| Capability::try_from(atom.to_owned()).unwrap())
            .collect()
    }

    #[test]
    fn icloud_sorts_on_the_client_though_it_advertises_sort() {
        // NOTE: iCloud's CAPABILITY line as of 2026-11-12.
        let icloud = capabilities(
            "XAPPLEPUSHSERVICE IMAP4 IMAP4rev1 CONDSTORE ENABLE QRESYNC QUOTA \
             XAPPLELITERAL NAMESPACE UIDPLUS UNSELECT SORT ESEARCH ESORT \
             CONTEXT=SORT WITHIN SASL-IR ID IDLE X-APPLE-REMOTE-LINKS LIST-STATUS",
        );

        assert!(default_sort_fallback(&icloud));
    }

    #[test]
    fn a_server_sorts_when_it_advertises_sort() {
        let fastmail = capabilities("IMAP4rev1 SORT SORT=DISPLAY XAPPLEPUSHSERVICE");
        let bare = capabilities("IMAP4rev1 IDLE");

        assert!(!default_sort_fallback(&fastmail));
        assert!(default_sort_fallback(&bare));
    }
}

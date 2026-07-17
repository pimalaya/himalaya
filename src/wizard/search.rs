//! Email-driven service discovery for the wizard.
//!
//! Mirrors the cardamum-android configuration screen, adapted to mail:
//! the address feeds io-pim-discovery's parallel discovery (fixed
//! provider rules, PACC, Mozilla autoconfig, RFC 6186 SRV, RFC 8620
//! JMAP resolve, with a final WWW-Authenticate probe refining the
//! advertised schemes), and every reachable service and authentication
//! method becomes one selectable entry. A detected Google or Microsoft
//! account collapses to its dedicated configurations (the proprietary
//! Gmail / Graph APIs plus IMAP+SMTP), matching the app's provider
//! short-circuit.

use std::{collections::BTreeSet, env, fmt};

use anyhow::Result;
use io_pim_discovery::{
    compose::{
        client::DiscoveryComposeClientStd,
        config::{
            DiscoveryAuthMethod, DiscoveryConfigSource, DiscoveryEndpoint, DiscoverySecurity,
            DiscoveryService, DiscoveryServiceConfig,
        },
        providers::DiscoveryKnownProvider,
    },
    shared::dns::system_resolver,
};
use pimalaya_stream::tls::{Rustls, Tls};
use url::Url;

/// DNS-over-TCP resolver backing discovery when `HIMALAYA_DNS_RESOLVER`
/// is unset and no system resolver is found: Cloudflare's `1.1.1.1`.
const DEFAULT_RESOLVER: &str = "tcp://1.1.1.1:53";

/// One selectable way to reach the account: a discovered service
/// paired with one of its authentication methods.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Discovered {
    pub kind: DiscoveredKind,
    /// Login hint advertised by the mechanism (usually the email).
    pub username: Option<String>,
    /// How to authenticate against the service.
    pub auth: DiscoveredAuth,
}

/// The discovered service kind, carrying its endpoint for the open
/// standards (the proprietary APIs have fixed endpoints).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiscoveredKind {
    /// An IMAP endpoint for receiving, paired with the SMTP endpoint
    /// for sending when one was discovered.
    ImapSmtp {
        imap: TcpEndpoint,
        smtp: Option<TcpEndpoint>,
    },
    /// A JMAP session endpoint (send and receive).
    Jmap(String),
    /// The Gmail REST API (Google accounts only).
    Gmail,
    /// The Microsoft Graph API (Microsoft accounts only).
    Msgraph,
}

/// A discovered TCP service endpoint (IMAP or SMTP).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TcpEndpoint {
    pub host: String,
    pub port: u16,
    pub security: DiscoverySecurity,
}

/// The discovered authentication method. OAuth 2.0 grants stay distinct
/// from bearer tokens: Himalaya reads a token an external manager (such
/// as Ortie) issues, but never runs a grant itself, so an OAuth entry
/// only points the user at that manager (see [`super::discover`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiscoveredAuth {
    Password,
    Token,
    OAuth,
}

impl fmt::Display for Discovered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let auth = match self.auth {
            DiscoveredAuth::Password => "Password",
            DiscoveredAuth::Token => "API token",
            DiscoveredAuth::OAuth => "OAuth 2.0",
        };

        match &self.kind {
            DiscoveredKind::ImapSmtp { imap, .. } => {
                write!(f, "IMAP + SMTP {} ({auth})", imap.host)
            }
            DiscoveredKind::Jmap(url) => write!(f, "JMAP {url} ({auth})"),
            DiscoveredKind::Gmail => write!(f, "Gmail API ({auth})"),
            DiscoveredKind::Msgraph => write!(f, "Microsoft Graph API ({auth})"),
        }
    }
}

impl Discovered {
    /// Best default login for the credential prompt: the advertised
    /// username when it looks like an address, else the searched email
    /// when the user typed a full one, else nothing (a bare domain,
    /// whose synthesized `@domain` form is rejected here).
    pub fn login_default(&self, email: &str) -> Option<String> {
        self.username
            .clone()
            .filter(|username| looks_like_address(username))
            .or_else(|| looks_like_address(email).then(|| email.to_string()))
    }

    /// Ranks an entry for the selection list: usable methods before
    /// OAuth (which cannot be configured here), then JMAP before
    /// IMAP+SMTP before the proprietary APIs, then API token before
    /// password.
    fn rank(&self) -> (u8, u8, u8) {
        let usable = match self.auth {
            DiscoveredAuth::OAuth => 1,
            _ => 0,
        };
        let service = match self.kind {
            DiscoveredKind::Jmap(_) => 0,
            DiscoveredKind::ImapSmtp { .. } => 1,
            DiscoveredKind::Gmail | DiscoveredKind::Msgraph => 2,
        };
        let auth = match self.auth {
            DiscoveredAuth::Token => 0,
            DiscoveredAuth::Password => 1,
            DiscoveredAuth::OAuth => 2,
        };

        (usable, service, auth)
    }
}

/// Searches every mail service reachable from `email` and returns one
/// selectable entry per service and authentication method, ordered by
/// [`Discovered::rank`]. A detected Google or Microsoft account yields
/// only its dedicated configurations.
pub fn search(email: &str) -> Result<Vec<Discovered>> {
    let client = DiscoveryComposeClientStd::new(discovery_resolver(), discovery_tls());
    let services = BTreeSet::from([
        DiscoveryService::Imap,
        DiscoveryService::Smtp,
        DiscoveryService::Jmap,
    ]);
    let configs = client.compose_all(email, services)?;

    let provider = provider_of(email, &configs);
    let mut found = Vec::new();

    // Google and Microsoft expose no JMAP: their dedicated set is
    // IMAP+SMTP plus a proprietary API, so JMAP is offered for other
    // providers only.
    if provider.is_none() {
        if let Some(jmap) = configs.iter().find(|c| c.service == DiscoveryService::Jmap) {
            if let DiscoveryEndpoint::Http(url) = &jmap.endpoint {
                let kind = DiscoveredKind::Jmap(url.clone());
                push_entries(&mut found, kind, jmap.username.clone(), &jmap.auth);
            }
        }
    }

    // A detected provider restricts IMAP+SMTP to its own configs, so
    // the app-style dedicated set shows instead of every discovered
    // relay.
    if let Some(imap) = best(&configs, DiscoveryService::Imap, provider) {
        if let Some(endpoint) = tcp_endpoint(imap) {
            let smtp = best(&configs, DiscoveryService::Smtp, provider).and_then(tcp_endpoint);
            let kind = DiscoveredKind::ImapSmtp {
                imap: endpoint,
                smtp,
            };
            push_entries(&mut found, kind, imap.username.clone(), &imap.auth);
        }
    }

    match provider {
        Some(DiscoveryKnownProvider::Google) => found.push(Discovered {
            kind: DiscoveredKind::Gmail,
            username: Some(email.to_string()),
            auth: DiscoveredAuth::Token,
        }),
        Some(DiscoveryKnownProvider::Microsoft) => found.push(Discovered {
            kind: DiscoveredKind::Msgraph,
            username: Some(email.to_string()),
            auth: DiscoveredAuth::Token,
        }),
        None => {}
    }

    found.sort_by_key(Discovered::rank);
    Ok(found)
}

/// Resolves the provider from the email domain (fast path for consumer
/// addresses), falling back to any provider-tagged config, which
/// catches custom domains detected through their MX records.
fn provider_of(email: &str, configs: &[DiscoveryServiceConfig]) -> Option<DiscoveryKnownProvider> {
    let by_domain = email
        .rsplit_once('@')
        .and_then(|(_, domain)| DiscoveryKnownProvider::from_domain(domain));

    by_domain.or_else(|| {
        configs.iter().find_map(|config| match config.source {
            DiscoveryConfigSource::Provider(provider) => Some(provider),
            _ => None,
        })
    })
}

/// Turns each authentication method of a service into one entry,
/// skipping duplicates (several OAuth grants collapse to one).
fn push_entries(
    found: &mut Vec<Discovered>,
    kind: DiscoveredKind,
    username: Option<String>,
    auth: &[DiscoveryAuthMethod],
) {
    for method in auth {
        let auth = match method {
            DiscoveryAuthMethod::Password => DiscoveredAuth::Password,
            DiscoveryAuthMethod::Bearer => DiscoveredAuth::Token,
            _ => DiscoveredAuth::OAuth,
        };

        let entry = Discovered {
            kind: kind.clone(),
            username: username.clone(),
            auth,
        };
        if !found.contains(&entry) {
            found.push(entry);
        }
    }
}

/// Picks the best config for a TCP service, restricted to the detected
/// provider's own configs when there is one: the most secure endpoint
/// wins, so a domain advertising both implicit TLS and STARTTLS keeps
/// the former.
fn best(
    configs: &[DiscoveryServiceConfig],
    service: DiscoveryService,
    provider: Option<DiscoveryKnownProvider>,
) -> Option<&DiscoveryServiceConfig> {
    configs
        .iter()
        .filter(|config| config.service == service)
        .filter(|config| match provider {
            Some(provider) => config.source == DiscoveryConfigSource::Provider(provider),
            None => true,
        })
        .max_by_key(|config| match &config.endpoint {
            DiscoveryEndpoint::Tcp {
                security: DiscoverySecurity::Tls,
                ..
            } => 2,
            DiscoveryEndpoint::Tcp {
                security: DiscoverySecurity::Starttls,
                ..
            } => 1,
            _ => 0,
        })
}

/// Whether a string is a full `local@domain` address (both parts
/// non-empty), rejecting the bare-domain `@domain` form.
fn looks_like_address(value: &str) -> bool {
    value
        .split_once('@')
        .is_some_and(|(local, domain)| !local.is_empty() && !domain.is_empty())
}

/// Extracts a [`TcpEndpoint`] from a config, or `None` for an HTTP one.
fn tcp_endpoint(config: &DiscoveryServiceConfig) -> Option<TcpEndpoint> {
    match &config.endpoint {
        DiscoveryEndpoint::Tcp {
            host,
            port,
            security,
        } => Some(TcpEndpoint {
            host: host.clone(),
            port: *port,
            security: *security,
        }),
        DiscoveryEndpoint::Http(_) => None,
    }
}

/// Resolver used by discovery: the `HIMALAYA_DNS_RESOLVER` override
/// first, then the system resolver (`/etc/resolv.conf` on unix, the
/// network adapters on windows), then the Cloudflare default. This
/// avoids leaking the email domain to a third-party resolver and works
/// around networks that block the default.
pub fn discovery_resolver() -> Url {
    if let Ok(resolver) = env::var("HIMALAYA_DNS_RESOLVER") {
        if let Ok(url) = resolver.parse() {
            return url;
        }
    }

    if let Some(url) = system_resolver() {
        return url;
    }

    DEFAULT_RESOLVER
        .parse()
        .expect("DEFAULT_RESOLVER must be a valid URL")
}

/// TLS profile for the HTTPS-bound discovery mechanisms; they only
/// speak HTTP/1.1 to `_well-known` endpoints.
fn discovery_tls() -> Tls {
    Tls {
        rustls: Rustls {
            alpn: vec!["http/1.1".into()],
            ..Default::default()
        },
        ..Default::default()
    }
}

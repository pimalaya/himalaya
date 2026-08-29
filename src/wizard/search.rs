//! # Service search
//!
//! Feeds an email address to io-pim-discovery's parallel discovery:
//! provider rules, PACC, Mozilla autoconfig, RFC 6186 SRV and RFC 8620
//! JMAP resolve, refined by a final `WWW-Authenticate` probe.
//!
//! Every reachable service becomes one selectable entry carrying the
//! authentication it advertised, the concrete method being picked once
//! the service is.
//!
//! A Google or Microsoft account collapses to its own configurations,
//! the proprietary API beside IMAP and SMTP.

use std::{collections::BTreeSet, env, fmt, time::Duration};

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

/// Upper bound on the parallel discovery fan-out. An unreachable
/// endpoint (a firewalled port, a black-hole host) must not stall the
/// interactive wizard, so mechanisms that have not reported by then are
/// abandoned and only what completed in time is offered.
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(8);

/// One selectable service to reach the account, carrying the
/// authentication capabilities it advertised. The concrete method (SASL
/// mechanism, HTTP scheme) is picked in a second prompt once the service
/// is chosen, so a service appears exactly once in the list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Discovered {
    pub kind: DiscoveredKind,
    /// Login hint advertised by the mechanism (usually the email).
    pub username: Option<String>,
    /// What the service accepts, folded across its discovered methods.
    pub auth: AuthCaps,
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

/// The authentication capabilities a service advertised, folded across
/// every method discovery found for it.
///
/// It drives the per-service prompt: which mechanisms or schemes to offer,
/// and whether the OAuth brokers appear.
///
/// Himalaya reads a token an external manager issues and runs no grant
/// itself, so OAuth is no method of its own here: it unlocks the brokers
/// behind the API token flow and nothing more.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AuthCaps {
    /// Basic/password auth: SASL PLAIN/LOGIN/SCRAM for IMAP+SMTP, Basic
    /// for JMAP. Often an app password (e.g. Fastmail, Gmail).
    pub basic: bool,
    /// A static bearer/API token: SASL OAUTHBEARER/XOAUTH2 for IMAP+SMTP,
    /// Bearer for JMAP.
    pub bearer: bool,
    /// An OAuth 2.0 grant is advertised, so a broker can issue the token.
    pub oauth: bool,
}

impl AuthCaps {
    /// Whether any capability was advertised. When none was (a mechanism
    /// that names no auth), the auth prompt offers every method so the
    /// user is never left without a choice.
    pub fn any(self) -> bool {
        self.basic || self.bearer || self.oauth
    }

    /// Whether a token (static or broker-issued) is on offer.
    pub fn token(self) -> bool {
        self.bearer || self.oauth
    }
}

impl fmt::Display for Discovered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            DiscoveredKind::ImapSmtp { imap, .. } => write!(f, "IMAP + SMTP {}", imap.host),
            DiscoveredKind::Jmap(url) => write!(f, "JMAP {url}"),
            DiscoveredKind::Gmail => write!(f, "Gmail API"),
            DiscoveredKind::Msgraph => write!(f, "Microsoft Graph API"),
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

    /// Ranks an entry for the selection list: JMAP first, then IMAP+SMTP,
    /// then the proprietary APIs.
    fn rank(&self) -> u8 {
        match self.kind {
            DiscoveredKind::Jmap(_) => 0,
            DiscoveredKind::ImapSmtp { .. } => 1,
            DiscoveredKind::Gmail | DiscoveredKind::Msgraph => 2,
        }
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
    let configs = client.compose_all_within(email, services, DISCOVERY_TIMEOUT)?;

    let provider = provider_of(email, &configs);
    let mut found = Vec::new();

    // NOTE: Google and Microsoft expose no JMAP, their set being IMAP and
    // SMTP beside a proprietary API.
    if provider.is_none()
        && let Some(jmap) = configs.iter().find(|c| c.service == DiscoveryService::Jmap)
        && let DiscoveryEndpoint::Http(url) = &jmap.endpoint
    {
        found.push(Discovered {
            kind: DiscoveredKind::Jmap(url.clone()),
            username: jmap.username.clone(),
            auth: caps_of(&jmap.auth),
        });
    }

    // NOTE: a detected provider restricts the entry to its own endpoints,
    // so its dedicated set shows rather than every discovered relay. The
    // two sides may advertise different auth, so the entry unions them.
    if let Some(imap) = best(&configs, DiscoveryService::Imap, provider)
        && let Some(endpoint) = tcp_endpoint(imap)
    {
        let smtp = best(&configs, DiscoveryService::Smtp, provider);
        let mut auth = caps_of(&imap.auth);
        if let Some(smtp) = smtp {
            let smtp_auth = caps_of(&smtp.auth);
            auth.basic |= smtp_auth.basic;
            auth.bearer |= smtp_auth.bearer;
            auth.oauth |= smtp_auth.oauth;
        }
        found.push(Discovered {
            kind: DiscoveredKind::ImapSmtp {
                imap: endpoint,
                smtp: smtp.and_then(tcp_endpoint),
            },
            username: imap.username.clone(),
            auth,
        });
    }

    match provider {
        Some(DiscoveryKnownProvider::Google) => found.push(Discovered {
            kind: DiscoveredKind::Gmail,
            username: Some(email.to_string()),
            auth: AuthCaps {
                oauth: true,
                ..Default::default()
            },
        }),
        Some(DiscoveryKnownProvider::Microsoft) => found.push(Discovered {
            kind: DiscoveredKind::Msgraph,
            username: Some(email.to_string()),
            auth: AuthCaps {
                oauth: true,
                ..Default::default()
            },
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

/// Folds a service's advertised methods into its [`AuthCaps`]: password
/// into `basic`, bearer into `bearer`, and every OAuth grant into `oauth`
/// (which only unlocks the token brokers, never a self-run grant).
fn caps_of(auth: &[DiscoveryAuthMethod]) -> AuthCaps {
    let mut caps = AuthCaps::default();

    for method in auth {
        match method {
            DiscoveryAuthMethod::Password => caps.basic = true,
            DiscoveryAuthMethod::Bearer => caps.bearer = true,
            _ => caps.oauth = true,
        }
    }

    caps
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

/// The resolver discovery queries: the `HIMALAYA_DNS_RESOLVER` override,
/// then the system one, then the Cloudflare default.
///
/// Preferring the system resolver keeps the email domain from leaking to a
/// third party, and the override works around a network blocking the
/// default.
pub fn discovery_resolver() -> Url {
    if let Ok(resolver) = env::var("HIMALAYA_DNS_RESOLVER")
        && let Ok(url) = resolver.parse()
    {
        return url;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_fold_each_method_onto_its_axis() {
        let oauth = DiscoveryAuthMethod::OauthIssuer("https://issuer".into());

        assert_eq!(
            caps_of(&[DiscoveryAuthMethod::Password]),
            AuthCaps {
                basic: true,
                ..Default::default()
            }
        );
        assert_eq!(
            caps_of(&[DiscoveryAuthMethod::Bearer]),
            AuthCaps {
                bearer: true,
                ..Default::default()
            }
        );
        assert_eq!(
            caps_of(&[oauth.clone()]),
            AuthCaps {
                oauth: true,
                ..Default::default()
            }
        );

        // NOTE: the Fastmail JMAP shape, a bearer and an OAuth grant with
        // no Basic, is one API token method whose brokers are unlocked.
        let fastmail = caps_of(&[DiscoveryAuthMethod::Bearer, oauth]);
        assert_eq!(
            fastmail,
            AuthCaps {
                bearer: true,
                oauth: true,
                ..Default::default()
            }
        );
        assert!(fastmail.token());
        assert!(!fastmail.basic);
    }

    #[test]
    fn caps_report_emptiness_and_token_offer() {
        assert!(!AuthCaps::default().any());
        assert!(!AuthCaps::default().token());

        let basic = AuthCaps {
            basic: true,
            ..Default::default()
        };
        assert!(basic.any());
        assert!(!basic.token());

        let oauth = AuthCaps {
            oauth: true,
            ..Default::default()
        };
        assert!(oauth.token());
    }
}

//! # Service search
//!
//! Feeds an address to io-pim-discovery's parallel discovery: provider
//! rules (by domain or MX), PACC, Mozilla autoconfig with its MX
//! fallback, RFC 6186 SRV and RFC 8620 JMAP resolve, refined by a final
//! `WWW-Authenticate` probe.
//!
//! What comes back is raw: [`crate::wizard::plan`] makes options of it.

use std::{collections::BTreeSet, env, time::Duration};

use anyhow::Result;
use io_pim_discovery::{
    compose::{
        client::DiscoveryComposeClientStd,
        config::{DiscoveryService, DiscoveryServiceConfig},
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

/// Searches the mail services reachable from `lookup`, an address or
/// `@<domain>`: IMAP, SMTP and JMAP, and the provider APIs a provider
/// rule offers.
///
/// A resolver that does not answer is an error.
pub fn search(lookup: &str) -> Result<Vec<DiscoveryServiceConfig>> {
    let client = DiscoveryComposeClientStd::new(discovery_resolver(), discovery_tls());

    let services = BTreeSet::from([
        DiscoveryService::Imap,
        DiscoveryService::Smtp,
        DiscoveryService::Jmap,
        DiscoveryService::Gmail,
        DiscoveryService::Msgraph,
    ]);

    Ok(client.compose_all_within(lookup, services, DISCOVERY_TIMEOUT)?)
}

/// The resolver discovery queries: the `HIMALAYA_DNS_RESOLVER` override,
/// then the system one, then the Cloudflare default.
///
/// Preferring the system resolver keeps the email domain from leaking to a
/// third party, and the override works around a network blocking the
/// default.
fn discovery_resolver() -> Url {
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

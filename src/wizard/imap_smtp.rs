//! IMAP + SMTP wizard.
//!
//! A discovery entry pins the endpoints, so [`configure_discovered`]
//! picks the SASL mechanism, prompts its credentials and tests the IMAP
//! connection, then, when discovery also found a submission endpoint,
//! asks whether SMTP shares them: if so the same credential backs both
//! sides, otherwise the SASL prompts run again for SMTP (IMAP and SMTP
//! may advertise different auth), and the SMTP connection is tested last.
//! The wizard never invents an SMTP host: with no discovered submission
//! endpoint the account is IMAP-only, and the user adds SMTP by hand.

use std::collections::HashMap;

use anyhow::{Result, bail};
use io_pim_discovery::compose::config::DiscoverySecurity;
use pimalaya_cli::{prompt, spinner::Spinner};
use pimalaya_stream::sasl::SaslMechanism;

use crate::{
    account::check,
    config::{
        ImapConfig, SaslAnonymousConfig, SaslConfig, SaslLoginConfig, SaslOauthbearerConfig,
        SaslPlainConfig, SaslScramSha256Config, SaslXoauth2Config, SmtpConfig,
    },
    wizard::{
        mailbox,
        search::{AuthCaps, Discovered, DiscoveredKind, TcpEndpoint},
        secret,
    },
};

// NOTE: the mechanisms split by credential kind, a password family
// (login + secret) and a token family (login + API token); ANONYMOUS
// carries none.
const PLAIN: &str = "PLAIN (username + password)";
const LOGIN: &str = "LOGIN (username + password)";
const SCRAM_SHA_256: &str = "SCRAM-SHA-256 (username + password)";
const ANONYMOUS: &str = "ANONYMOUS (no credentials)";
const OAUTHBEARER: &str = "OAUTHBEARER (username + API token)";
const XOAUTH2: &str = "XOAUTH2 (username + API token)";

/// Configures IMAP + SMTP from a discovered entry: pick the SASL
/// mechanism and credentials for IMAP, test the connection, then ask
/// whether SMTP reuses them — configuring a distinct SASL when it does
/// not — and test SMTP last. Both connections are validated here, so the
/// caller skips the final account test. Returns the discovered
/// `mailbox.alias.*` entries (the IMAP inbox) alongside the configs.
pub fn configure_discovered(
    account_name: &str,
    email: &str,
    discovered: &Discovered,
) -> Result<(ImapConfig, Option<SmtpConfig>, HashMap<String, String>)> {
    let DiscoveredKind::ImapSmtp { imap, smtp } = &discovered.kind else {
        bail!("Expected an IMAP + SMTP configuration");
    };

    let login_hint = discovered.login_default(email);

    // Probe the server so only the mechanisms it actually advertises are
    // offered (LOGIN last); on any probe failure fall back to the full
    // list keyed on what discovery advertised.
    let probed = probe_imap_mechanisms(
        &endpoint_server(imap),
        imap.security == DiscoverySecurity::Starttls,
    );
    let imap_sasl = prompt_sasl(
        account_name,
        login_hint.as_deref(),
        discovered.auth,
        probed.as_deref(),
    )?;
    let imap = imap_config(imap, imap_sasl.clone());
    test_connection("IMAP", || check::connect_imap(&imap))?;

    // NOTE: IMAP has no reliable special-use listing yet (see the mailbox
    // module), so only the always-present INBOX is pinned as the default.
    let aliases = mailbox::imap_aliases();

    // The wizard never invents an SMTP host: when discovery found no
    // submission endpoint, the account stays IMAP-only and the user adds
    // SMTP by hand. Otherwise configure and test it, reusing the IMAP
    // credential unless the user opts for a distinct one — IMAP and SMTP
    // may advertise different auth. SMTP advertises its auth over EHLO,
    // not the IMAP CAPABILITY probe, so its mechanism list stays keyed on
    // discovery (probed = None), unlike the IMAP side above.
    let smtp = match smtp {
        Some(endpoint) => {
            let smtp_sasl = if prompt::bool("Use the same credentials for SMTP?", true)? {
                imap_sasl
            } else {
                prompt_sasl(account_name, login_hint.as_deref(), discovered.auth, None)?
            };
            let smtp = smtp_config(endpoint, smtp_sasl);
            test_connection("SMTP", || check::connect_smtp(&smtp))?;
            Some(smtp)
        }
        None => None,
    };

    Ok((imap, smtp, aliases))
}

/// Runs a connection `test` behind a labelled spinner, surfacing a
/// failure as the wizard's error (like the final account test) so a bad
/// credential stops here instead of yielding a config that cannot
/// connect.
fn test_connection(label: &str, test: impl FnOnce() -> Result<()>) -> Result<()> {
    let spinner = Spinner::start(format!("Testing {label} connection"));

    if let Err(err) = test() {
        spinner.failure(format!("{label} connection failed"));
        return Err(err);
    }

    spinner.success(format!("{label} connection succeeded"));
    Ok(())
}

/// Prompts the SASL mechanism then its credentials. When `probed` is
/// `Some` (a live IMAP CAPABILITY probe) only those mechanisms are
/// offered, most preferred first and LOGIN last; otherwise the full list
/// keyed on `caps` is offered, so a failed probe never leaves the user
/// stuck. The token mechanisms' OAuth brokers appear only when a token
/// or OAuth grant was advertised.
fn prompt_sasl(
    account_name: &str,
    login_hint: Option<&str>,
    caps: AuthCaps,
    probed: Option<&[SaslMechanism]>,
) -> Result<SaslConfig> {
    let mechanism = prompt_mechanism(caps, probed)?;
    build_sasl(mechanism, account_name, login_hint, caps)
}

/// Prompts the authentication mechanism: the probed list when the server
/// advertised one, otherwise the full fallback list. A single candidate
/// is selected without prompting.
fn prompt_mechanism(caps: AuthCaps, probed: Option<&[SaslMechanism]>) -> Result<SaslMechanism> {
    let mechanisms = match probed {
        Some(mechanisms) if !mechanisms.is_empty() => mechanisms.to_vec(),
        _ => fallback_mechanisms(caps),
    };

    let labels: Vec<&str> = mechanisms.iter().map(mechanism_label).collect();
    let label = if labels.len() == 1 {
        labels[0]
    } else {
        prompt::item("SASL mechanism:", labels, None)?
    };

    // Labels are unique, so the chosen one maps back to exactly one
    // mechanism.
    Ok(mechanisms
        .into_iter()
        .find(|m| mechanism_label(m) == label)
        .expect("chosen label matches a mechanism"))
}

/// Prompts the credentials for `mechanism` and builds its SASL config.
/// ANONYMOUS carries no login; every other mechanism needs one, plus a
/// password (basic family) or an API token (OAuth family).
fn build_sasl(
    mechanism: SaslMechanism,
    account_name: &str,
    login_hint: Option<&str>,
    caps: AuthCaps,
) -> Result<SaslConfig> {
    if let SaslMechanism::Anonymous = mechanism {
        let message = prompt::text("ANONYMOUS message (optional):", None::<&str>)?;
        let message = Some(message).filter(|m| !m.trim().is_empty());
        return Ok(SaslConfig::Anonymous(SaslAnonymousConfig { message }));
    }

    let login = prompt::text("Login:", login_hint)?;

    Ok(match mechanism {
        SaslMechanism::Plain => {
            let passwd = secret::configure_password("Password", account_name)?;
            SaslConfig::Plain(SaslPlainConfig {
                authzid: None,
                authcid: login,
                passwd,
            })
        }
        SaslMechanism::Login => {
            let password = secret::configure_password("Password", account_name)?;
            SaslConfig::Login(SaslLoginConfig {
                username: login,
                password,
            })
        }
        SaslMechanism::ScramSha256 => {
            let password = secret::configure_password("Password", account_name)?;
            SaslConfig::ScramSha256(SaslScramSha256Config {
                username: login,
                password,
            })
        }
        SaslMechanism::OAuthBearer => {
            let token =
                secret::configure_token("API token", account_name, caps.oauth || !caps.any())?;
            SaslConfig::Oauthbearer(SaslOauthbearerConfig {
                username: login,
                token,
            })
        }
        SaslMechanism::XOAuth2 => {
            let token =
                secret::configure_token("API token", account_name, caps.oauth || !caps.any())?;
            SaslConfig::Xoauth2(SaslXoauth2Config {
                username: login,
                token,
            })
        }
        SaslMechanism::Anonymous => unreachable!("handled above"),
    })
}

/// The menu label for a mechanism, split by the credential it needs.
fn mechanism_label(mechanism: &SaslMechanism) -> &'static str {
    match mechanism {
        SaslMechanism::ScramSha256 => SCRAM_SHA_256,
        SaslMechanism::Plain => PLAIN,
        SaslMechanism::OAuthBearer => OAUTHBEARER,
        SaslMechanism::XOAuth2 => XOAUTH2,
        SaslMechanism::Anonymous => ANONYMOUS,
        SaslMechanism::Login => LOGIN,
    }
}

/// The mechanisms offered when no live probe is available, keyed on what
/// discovery advertised (every family when nothing was): most preferred
/// first, LOGIN last, token mechanisms only when a token or OAuth grant
/// was advertised.
fn fallback_mechanisms(caps: AuthCaps) -> Vec<SaslMechanism> {
    let mut mechanisms = Vec::new();

    if caps.basic || !caps.any() {
        mechanisms.extend([SaslMechanism::ScramSha256, SaslMechanism::Plain]);
    }
    if caps.token() || !caps.any() {
        mechanisms.extend([SaslMechanism::OAuthBearer, SaslMechanism::XOAuth2]);
    }
    if caps.basic || !caps.any() {
        mechanisms.extend([SaslMechanism::Anonymous, SaslMechanism::Login]);
    }

    mechanisms
}

/// Probes the IMAP server for the mechanisms it advertises, returning
/// `None` (offer the full list) when the probe fails or advertises
/// nothing usable. The error is logged, never surfaced: the wizard falls
/// back rather than stopping.
fn probe_imap_mechanisms(server: &str, starttls: bool) -> Option<Vec<SaslMechanism>> {
    match check::probe_imap_mechanisms(server, starttls) {
        Ok(mechanisms) if !mechanisms.is_empty() => Some(mechanisms),
        Ok(_) => None,
        Err(err) => {
            log::warn!("could not probe IMAP capabilities, offering all mechanisms: {err:#}");
            None
        }
    }
}

/// The `scheme://host:port` string for a discovered endpoint, matching
/// how [`imap_config`] builds the server URL.
fn endpoint_server(endpoint: &TcpEndpoint) -> String {
    let scheme = if endpoint.security == DiscoverySecurity::Tls {
        "imaps"
    } else {
        "imap"
    };

    format!("{scheme}://{}:{}", endpoint.host, endpoint.port)
}

fn imap_config(endpoint: &TcpEndpoint, sasl: SaslConfig) -> ImapConfig {
    ImapConfig {
        server: endpoint_server(endpoint),
        tls: Default::default(),
        starttls: endpoint.security == DiscoverySecurity::Starttls,
        alpn: io_imap::client::default_alpn(),
        sasl: Some(sasl),
        id: Default::default(),
        sort: Default::default(),
    }
}

fn smtp_config(endpoint: &TcpEndpoint, sasl: SaslConfig) -> SmtpConfig {
    let scheme = if endpoint.security == DiscoverySecurity::Tls {
        "smtps"
    } else {
        "smtp"
    };

    SmtpConfig {
        server: format!("{scheme}://{}:{}", endpoint.host, endpoint.port),
        tls: Default::default(),
        starttls: endpoint.security == DiscoverySecurity::Starttls,
        alpn: io_smtp::client::SmtpClientStd::default_alpn(),
        sasl: Some(sasl),
    }
}

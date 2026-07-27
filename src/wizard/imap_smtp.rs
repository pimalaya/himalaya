//! IMAP + SMTP wizard.
//!
//! A discovery entry pins the endpoints, so [`configure_discovered`]
//! picks the SASL mechanism, prompts its credentials and tests the IMAP
//! connection, then asks whether SMTP shares them: if so the same
//! credential backs both sides, otherwise the SASL prompts run again for
//! SMTP (IMAP and SMTP may advertise different auth). The SMTP connection
//! is tested last. [`configure_manual`] is the fallback when discovery
//! finds nothing or the user typed an `imap://` URL: the full
//! per-protocol prompts run, seeded from what is known.

use std::collections::HashMap;

use anyhow::{Result, bail};
use io_pim_discovery::compose::config::DiscoverySecurity;
use pimalaya_cli::{
    prompt,
    spinner::Spinner,
    wizard::imap::{
        self as imap_wizard, Encryption as ImapEncryption, ImapAuth, ImapSecret, WizardImapConfig,
    },
    wizard::smtp::{self as smtp_wizard, Encryption as SmtpEncryption},
};
use pimalaya_stream::sasl::SaslMechanism;
use url::Url;

use crate::{
    account::check,
    config::{
        ImapConfig, SaslAnonymousConfig, SaslConfig, SaslLoginConfig, SaslOauthbearerConfig,
        SaslPlainConfig, SaslScramSha256Config, SaslXoauth2Config, SmtpConfig,
    },
    wizard::{
        account::{imap_to_config, smtp_to_config},
        mailbox,
        search::{AuthCaps, Discovered, DiscoveredKind, TcpEndpoint},
        secret,
    },
};

// NOTE: the mechanisms split by credential kind, a password family
// (login + secret) and a token family (login + API token); ANONYMOUS
// carries none.
const PLAIN: &str = "PLAIN (login + password)";
const LOGIN: &str = "LOGIN (login + password)";
const SCRAM_SHA_256: &str = "SCRAM-SHA-256 (login + password)";
const ANONYMOUS: &str = "ANONYMOUS (no credentials)";
const OAUTHBEARER: &str = "OAUTHBEARER (login + API token)";
const XOAUTH2: &str = "XOAUTH2 (login + API token)";

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
) -> Result<(ImapConfig, SmtpConfig, HashMap<String, String>)> {
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

    // NOTE: IMAP and SMTP may advertise different auth, so SMTP either
    // reuses the IMAP credential or configures a distinct one.
    let smtp_endpoint = smtp.clone().unwrap_or_else(|| default_smtp(email));
    // NOTE: SMTP advertises its auth over EHLO, not the IMAP CAPABILITY
    // probe, so its mechanism list stays keyed on discovery (probed =
    // None), unlike the IMAP side above.
    let smtp_sasl = if prompt::bool("Use the same credentials for SMTP?", true)? {
        imap_sasl
    } else {
        prompt_sasl(account_name, login_hint.as_deref(), discovered.auth, None)?
    };
    let smtp = smtp_config(&smtp_endpoint, smtp_sasl);
    test_connection("SMTP", || check::connect_smtp(&smtp))?;

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

/// Runs the full per-protocol IMAP and SMTP prompts, seeding IMAP from
/// `imap_url` when the user typed one.
pub fn configure_manual(
    account_name: &str,
    local_part: &str,
    domain: &str,
    imap_url: Option<&Url>,
) -> Result<(ImapConfig, SmtpConfig)> {
    let imap_default = imap_url.map(seed_imap);
    let imap = imap_wizard::run(account_name, local_part, domain, imap_default.as_ref())?;

    // Probe the entered server and pick a mechanism it advertises (LOGIN
    // last) instead of assuming PLAIN; the secret the wizard collected
    // backs whichever mechanism is chosen. No discovery ran here, so the
    // fallback list is the full one.
    let (server, starttls) = wizard_imap_server(&imap);
    let probed = probe_imap_mechanisms(&server, starttls);
    let mechanism = prompt_mechanism(AuthCaps::default(), probed.as_deref())?;

    let imap_host = imap.host.clone();
    let imap = imap_to_config(imap, mechanism)?;

    // Like the discovered flow, offer to reuse the IMAP credentials for
    // SMTP so they are entered once. The endpoint is still prompted
    // (nothing was discovered), seeded from the IMAP host.
    let smtp = if prompt::bool("Use the same credentials for SMTP?", true)? {
        let sasl = imap
            .sasl
            .clone()
            .expect("manual IMAP flow always sets a SASL config");
        prompt_smtp_endpoint(&imap_host, sasl)?
    } else {
        let smtp = smtp_wizard::run(account_name, local_part, domain, None)?;
        smtp_to_config(smtp)?
    };

    Ok((imap, smtp))
}

/// Prompts only the SMTP endpoint (host, encryption, port) and builds
/// its config reusing `sasl`, the IMAP credentials, so the manual flow
/// does not re-enter them. The host defaults to the IMAP host, the
/// common case when a provider shares one hostname for both.
fn prompt_smtp_endpoint(imap_host: &str, sasl: SaslConfig) -> Result<SmtpConfig> {
    let host = prompt::text("SMTP hostname:", Some(imap_host))?;

    let encryptions = [
        SmtpEncryption::Tls,
        SmtpEncryption::StartTls,
        SmtpEncryption::None,
    ];
    let encryption = prompt::item("SMTP encryption:", encryptions, Some(SmtpEncryption::Tls))?;

    let default_port = match encryption {
        SmtpEncryption::Tls => 465,
        SmtpEncryption::StartTls => 587,
        SmtpEncryption::None => 25,
    };
    let port = prompt::u16("SMTP port:", Some(default_port))?;

    let scheme = match encryption {
        SmtpEncryption::Tls => "smtps",
        SmtpEncryption::StartTls | SmtpEncryption::None => "smtp",
    };

    Ok(SmtpConfig {
        server: format!("{scheme}://{host}:{port}"),
        tls: Default::default(),
        starttls: matches!(encryption, SmtpEncryption::StartTls),
        alpn: io_smtp::client::SmtpClientStd::default_alpn(),
        sasl: Some(sasl),
    })
}

/// Fallback SMTP endpoint when discovery found IMAP but no SMTP:
/// `smtp.<domain>` over implicit TLS.
fn default_smtp(email: &str) -> TcpEndpoint {
    let domain = email.rsplit_once('@').map(|(_, d)| d).unwrap_or(email);

    TcpEndpoint {
        host: format!("smtp.{domain}"),
        port: 465,
        security: DiscoverySecurity::Tls,
    }
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

/// The `scheme://host:port` string and STARTTLS flag for a manually
/// entered IMAP config, matching how [`imap_to_config`] builds its
/// server URL, so the probe hits the same endpoint that gets saved.
fn wizard_imap_server(config: &WizardImapConfig) -> (String, bool) {
    let scheme = match config.encryption {
        ImapEncryption::Tls => "imaps",
        ImapEncryption::StartTls | ImapEncryption::None => "imap",
    };
    let server = format!("{scheme}://{}:{}", config.host, config.port);
    let starttls = matches!(config.encryption, ImapEncryption::StartTls);

    (server, starttls)
}

/// Seeds the manual IMAP prompts from a typed `imap://` / `imaps://`
/// URL; the login and secret are left for the user.
fn seed_imap(url: &Url) -> WizardImapConfig {
    let encryption = match url.scheme() {
        "imaps" => ImapEncryption::Tls,
        _ => ImapEncryption::StartTls,
    };
    let port = url.port().unwrap_or(match encryption {
        ImapEncryption::Tls => 993,
        _ => 143,
    });

    WizardImapConfig {
        host: url.host_str().unwrap_or_default().to_string(),
        port,
        encryption,
        login: String::new(),
        auth: ImapAuth::Password(ImapSecret::Raw(String::new().into())),
    }
}

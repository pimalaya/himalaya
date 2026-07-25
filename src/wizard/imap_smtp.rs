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
    wizard::smtp::{self as smtp_wizard},
};
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

// SASL mechanisms Himalaya supports, split by credential kind: the
// password family (login + secret) and the token family (login + API
// token). ANONYMOUS carries no credential.
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

    // Receiving side: configure, then validate before moving on.
    let imap_sasl = prompt_sasl(account_name, login_hint.as_deref(), discovered.auth)?;
    let imap = imap_config(imap, imap_sasl.clone());
    test_connection("IMAP", || check::connect_imap(&imap))?;

    // IMAP has no reliable special-use listing yet (see [`mailbox`]), so
    // only the always-present INBOX is pinned as the default mailbox.
    let aliases = mailbox::imap_aliases();

    // Sending side: reuse the IMAP credential unless the user opts to
    // configure a distinct one (IMAP and SMTP may advertise different
    // auth), then validate it too.
    let smtp_endpoint = smtp.clone().unwrap_or_else(|| default_smtp(email));
    let smtp_sasl = if prompt::bool("Use the same credentials for SMTP?", true)? {
        imap_sasl
    } else {
        prompt_sasl(account_name, login_hint.as_deref(), discovered.auth)?
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

/// Prompts the SASL mechanism from `caps` (every mechanism offered when
/// none was advertised), then its credentials. The token mechanisms and
/// their OAuth brokers appear only when a token or OAuth grant was
/// advertised.
fn prompt_sasl(account_name: &str, login_hint: Option<&str>, caps: AuthCaps) -> Result<SaslConfig> {
    let mut mechs = Vec::new();
    if caps.basic || !caps.any() {
        mechs.extend([PLAIN, LOGIN, SCRAM_SHA_256, ANONYMOUS]);
    }
    if caps.token() || !caps.any() {
        mechs.extend([OAUTHBEARER, XOAUTH2]);
    }

    let mech = if mechs.len() == 1 {
        mechs[0]
    } else {
        prompt::item("SASL mechanism:", mechs, None)?
    };

    // ANONYMOUS carries no login; every other mechanism needs one.
    if mech == ANONYMOUS {
        let message = prompt::text("ANONYMOUS message (optional):", None::<&str>)?;
        let message = Some(message).filter(|m| !m.trim().is_empty());
        return Ok(SaslConfig::Anonymous(SaslAnonymousConfig { message }));
    }

    let login = prompt::text("Login:", login_hint)?;

    Ok(match mech {
        PLAIN => {
            let passwd = secret::configure_password("Password", account_name)?;
            SaslConfig::Plain(SaslPlainConfig {
                authzid: None,
                authcid: login,
                passwd,
            })
        }
        LOGIN => {
            let password = secret::configure_password("Password", account_name)?;
            SaslConfig::Login(SaslLoginConfig {
                username: login,
                password,
            })
        }
        SCRAM_SHA_256 => {
            let password = secret::configure_password("Password", account_name)?;
            SaslConfig::ScramSha256(SaslScramSha256Config {
                username: login,
                password,
            })
        }
        OAUTHBEARER => {
            let token =
                secret::configure_token("API token", account_name, caps.oauth || !caps.any())?;
            SaslConfig::Oauthbearer(SaslOauthbearerConfig {
                username: login,
                token,
            })
        }
        XOAUTH2 => {
            let token =
                secret::configure_token("API token", account_name, caps.oauth || !caps.any())?;
            SaslConfig::Xoauth2(SaslXoauth2Config {
                username: login,
                token,
            })
        }
        _ => unreachable!(),
    })
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
    let smtp = smtp_wizard::run(account_name, local_part, domain, None)?;

    Ok((imap_to_config(imap)?, smtp_to_config(smtp)?))
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
    let scheme = if endpoint.security == DiscoverySecurity::Tls {
        "imaps"
    } else {
        "imap"
    };

    ImapConfig {
        server: format!("{scheme}://{}:{}", endpoint.host, endpoint.port),
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

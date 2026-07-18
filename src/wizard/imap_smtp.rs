//! IMAP + SMTP wizard.
//!
//! A discovery entry pins the endpoints and the authentication method,
//! so [`configure_discovered`] prompts only the login and the secret,
//! then the same credential configures both the receiving (IMAP) and
//! the sending (SMTP) side. [`configure_manual`] is the fallback when
//! discovery finds nothing or the user typed an `imap://` URL: the full
//! per-protocol prompts run, seeded from what is known.

use anyhow::{Result, bail};
use io_pim_discovery::compose::config::DiscoverySecurity;
use pimalaya_cli::{
    prompt,
    wizard::imap::{
        self as imap_wizard, Encryption as ImapEncryption, ImapAuth, ImapSecret, WizardImapConfig,
    },
    wizard::smtp::{self as smtp_wizard},
};
use url::Url;

use crate::{
    config::{ImapConfig, SaslConfig, SaslOauthbearerConfig, SaslPlainConfig, SmtpConfig},
    wizard::{
        account::{imap_to_config, smtp_to_config},
        search::{Discovered, DiscoveredAuth, DiscoveredKind, TcpEndpoint},
        secret,
    },
};

/// Configures IMAP + SMTP from a discovered entry: the login (defaulted
/// to the email) and the secret are prompted, and the same credential
/// backs both protocols.
pub fn configure_discovered(
    account_name: &str,
    email: &str,
    discovered: &Discovered,
) -> Result<(ImapConfig, SmtpConfig)> {
    let DiscoveredKind::ImapSmtp { imap, smtp } = &discovered.kind else {
        bail!("Expected an IMAP + SMTP configuration");
    };

    let default_login = discovered.login_default(email);
    let login = prompt::text("Login:", default_login.as_deref())?;

    // IMAP and SMTP share one credential on the discovered path, so key
    // it by the account alone rather than per protocol.
    let sasl = match discovered.auth {
        DiscoveredAuth::Password => {
            let passwd = secret::configure("Password", account_name, &[])?;
            SaslConfig::Plain(SaslPlainConfig {
                authzid: None,
                authcid: login,
                passwd,
            })
        }
        DiscoveredAuth::Token => {
            let token = secret::configure("API token", account_name, &secret::ortie(account_name))?;
            SaslConfig::Oauthbearer(SaslOauthbearerConfig {
                username: login,
                token,
            })
        }
        DiscoveredAuth::OAuth => bail!("OAuth 2.0 cannot be configured directly"),
    };

    let smtp = smtp.clone().unwrap_or_else(|| default_smtp(email));

    Ok((imap_config(imap, sasl.clone()), smtp_config(&smtp, sasl)))
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

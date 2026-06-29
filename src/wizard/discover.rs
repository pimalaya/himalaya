//! Interactive configuration wizard with discovery-based defaults.
//!
//! Triggered by `cli::load_or_wizard` when no config file is found.
//!
//! Flow:
//!
//! 1. Confirm with the user; exit cleanly if they decline.
//! 2. Ask for an email address (account name defaults from it).
//! 3. Run PACC, Autoconfig (which does the MX hop for custom domains)
//!    and RFC 6186 SRV, merging their results across mechanisms.
//! 4. Detect the provider (Google / Microsoft / other) and offer the
//!    relevant backends: the API path (Gmail / Microsoft Graph) and/or
//!    IMAP+SMTP and/or JMAP.
//! 5. Run the matching sub-wizard, build a [`Config`], write it.

use std::{collections::HashMap, env, fmt, path::Path};

use anyhow::{Result, anyhow};
use pimalaya_cli::{
    prompt,
    wizard::{
        imap::{
            self as imap_wizard, Encryption as ImapEncryption, ImapAuth, ImapSecret,
            WizardImapConfig,
        },
        jmap::{self as jmap_wizard, WizardJmapConfig},
        smtp::{
            self as smtp_wizard, Encryption as SmtpEncryption, SmtpAuth, SmtpSecret,
            WizardSmtpConfig,
        },
    },
};
use pimalaya_config::{command::shell, secret::Secret};
use pimalaya_stream::tls::Tls;
use pimconf::autoconfig::types::Autoconfig;
use url::Url;

use crate::{
    config::{
        AccountConfig, Config, GmailAuthConfig, GmailConfig, MsgraphAuthConfig, MsgraphConfig,
    },
    wizard::{
        account::{imap_to_config, jmap_to_config, smtp_to_config},
        autoconfig, pacc, srv,
    },
};

/// Default DNS resolver used by PACC, Autoconfig, and SRV discovery
/// when `HIMALAYA_DNS_RESOLVER` is unset. Cloudflare's `1.1.1.1`.
const DEFAULT_RESOLVER: &str = "tcp://1.1.1.1:53";

/// Resolver used by discovery, overridable via `HIMALAYA_DNS_RESOLVER`
/// so users can avoid leaking the email domain to a third-party
/// resolver or work around networks that block the default.
pub fn discovery_resolver() -> Url {
    let resolver =
        env::var("HIMALAYA_DNS_RESOLVER").unwrap_or_else(|_| DEFAULT_RESOLVER.to_string());

    resolver.parse().unwrap_or_else(|_| {
        DEFAULT_RESOLVER
            .parse()
            .expect("DEFAULT_RESOLVER must be a valid URL")
    })
}

/// Builds the [`Tls`] profile passed to the per-mechanism discovery
/// clients via `with_tls`. Discovery only speaks HTTPS to `_well-known`
/// endpoints, so `http/1.1` is the only ALPN protocol we offer.
pub fn discovery_tls() -> Tls {
    let mut tls = Tls::default();
    tls.rustls.alpn = vec!["http/1.1".into()];
    tls
}

/// Per-protocol backend configs surfaced by autodiscovery.
#[derive(Default)]
pub struct DiscoveryResult {
    pub jmap: Option<WizardJmapConfig>,
    pub imap: Option<WizardImapConfig>,
    pub smtp: Option<WizardSmtpConfig>,
}

impl DiscoveryResult {
    /// Fills this result's empty slots from `other`, so an earlier
    /// mechanism's hit is kept and later mechanisms only complete what
    /// is still missing.
    fn merge(&mut self, other: DiscoveryResult) {
        self.imap = self.imap.take().or(other.imap);
        self.smtp = self.smtp.take().or(other.smtp);
        self.jmap = self.jmap.take().or(other.jmap);
    }
}

/// The email provider, used to decide which backends to offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Provider {
    Google,
    Microsoft,
    Other,
}

/// A backend the wizard can configure, rendered in the selection menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BackendChoice {
    ImapSmtp,
    Jmap,
    GmailApi,
    MsgraphApi,
}

impl fmt::Display for BackendChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::ImapSmtp => "IMAP + SMTP",
            Self::Jmap => "JMAP",
            Self::GmailApi => "Gmail API (OAuth)",
            Self::MsgraphApi => "Microsoft Graph API (OAuth)",
        };
        f.write_str(label)
    }
}

/// Runs the wizard, writing the resulting [`Config`] to `target`.
/// Returns `Ok(None)` when the user declines to create one.
pub fn run(target: &Path) -> Result<Option<Config>> {
    let confirm = format!(
        "No configuration found. Create one at {}?",
        target.display()
    );
    if !prompt::bool(&confirm, true)? {
        return Ok(None);
    }

    let email = prompt::text::<&str>("Email address:", None)?;
    let (local_part, domain) = email
        .rsplit_once('@')
        .filter(|(local, domain)| !local.is_empty() && !domain.is_empty())
        .ok_or_else(|| anyhow!("Invalid email address `{email}`: expected `local@domain`"))?;

    let account_name = prompt::text("Account name:", Some(local_part))?;

    let (discovery, provider) = discover(local_part, domain);
    let account = build_account(
        &account_name,
        &email,
        local_part,
        domain,
        discovery,
        provider,
    )?;

    let config = Config {
        accounts: HashMap::from([(account_name, account)]),
        ..Default::default()
    };

    config.write(target)?;
    println!("Configuration written to {}.", target.display());

    Ok(Some(config))
}

/// Runs every discovery mechanism and merges their results, then
/// classifies the provider from the autoconfig response.
fn discover(local_part: &str, domain: &str) -> (DiscoveryResult, Provider) {
    let mut result = DiscoveryResult::default();

    if let Some(config) = pacc::run(domain) {
        result.merge(pacc::defaults(&config));
    }

    let autoconfig = autoconfig::run(local_part, domain);
    if let Some(config) = &autoconfig {
        result.merge(autoconfig::defaults(config));
    }

    if let Some(report) = srv::run(domain) {
        result.merge(srv::defaults(&report));
    }

    let provider = detect_provider(domain, autoconfig.as_ref());
    (result, provider)
}

/// Prompts for a backend (when more than one fits), runs the matching
/// sub-wizard and assembles the [`AccountConfig`].
fn build_account(
    account_name: &str,
    email: &str,
    local_part: &str,
    domain: &str,
    discovery: DiscoveryResult,
    provider: Provider,
) -> Result<AccountConfig> {
    let choice = choose_backend(&discovery, provider)?;
    let DiscoveryResult { jmap, imap, smtp } = discovery;

    let account = match choice {
        BackendChoice::GmailApi => AccountConfig {
            default: true,
            gmail: Some(gmail_account()?),
            ..Default::default()
        },
        BackendChoice::MsgraphApi => AccountConfig {
            default: true,
            msgraph: Some(msgraph_account()?),
            ..Default::default()
        },
        BackendChoice::Jmap => {
            let jmap = jmap_wizard::run(account_name, local_part, domain, jmap.as_ref())?;
            AccountConfig {
                default: true,
                jmap: Some(jmap_to_config(jmap)?),
                ..Default::default()
            }
        }
        BackendChoice::ImapSmtp => {
            if matches!(provider, Provider::Google | Provider::Microsoft) {
                println!(
                    "Note: {} requires an app-specific password for IMAP/SMTP, not your account password.",
                    provider_label(provider),
                );
            }

            let imap_default = imap.or_else(|| provider_imap_default(provider, email));
            let smtp_default = smtp.or_else(|| provider_smtp_default(provider, email));

            let imap = imap_wizard::run(account_name, local_part, domain, imap_default.as_ref())?;
            let smtp = smtp_wizard::run(account_name, local_part, domain, smtp_default.as_ref())?;

            AccountConfig {
                default: true,
                imap: Some(imap_to_config(imap)?),
                smtp: Some(smtp_to_config(smtp)?),
                ..Default::default()
            }
        }
    };

    Ok(account)
}

/// Returns the backend to configure, prompting only when the provider
/// and discovery leave more than one sensible option.
fn choose_backend(discovery: &DiscoveryResult, provider: Provider) -> Result<BackendChoice> {
    let choices = backend_choices(discovery, provider);

    if let [only] = choices.as_slice() {
        return Ok(*only);
    }

    let choice = prompt::item("Which backend would you like to configure?", choices, None)?;
    Ok(choice)
}

/// The backend options offered for a given provider and discovery.
fn backend_choices(discovery: &DiscoveryResult, provider: Provider) -> Vec<BackendChoice> {
    match provider {
        Provider::Google => vec![BackendChoice::ImapSmtp, BackendChoice::GmailApi],
        Provider::Microsoft => vec![BackendChoice::MsgraphApi, BackendChoice::ImapSmtp],
        Provider::Other => {
            let mut choices = Vec::new();

            if discovery.imap.is_some() || discovery.smtp.is_some() {
                choices.push(BackendChoice::ImapSmtp);
            }
            if discovery.jmap.is_some() {
                choices.push(BackendChoice::Jmap);
            }

            // Nothing discovered: fall back to manual IMAP+SMTP entry.
            if choices.is_empty() {
                choices.push(BackendChoice::ImapSmtp);
            }

            choices
        }
    }
}

/// Builds a Gmail config block, explaining that the OAuth token is the
/// user's responsibility.
fn gmail_account() -> Result<GmailConfig> {
    print_oauth_notice("Gmail", "gmail");

    let user_id = prompt::text("Gmail user id:", Some("me"))?;
    let token = oauth_token_secret()?;

    Ok(GmailConfig {
        user_id,
        tls: Default::default(),
        alpn: vec!["http/1.1".to_string()],
        auth: GmailAuthConfig { token },
    })
}

/// Builds a Microsoft Graph config block, explaining that the OAuth
/// token is the user's responsibility.
fn msgraph_account() -> Result<MsgraphConfig> {
    print_oauth_notice("Microsoft Graph", "msgraph");

    let user_id = prompt::text("Microsoft Graph user id:", Some("me"))?;
    let token = oauth_token_secret()?;

    Ok(MsgraphConfig {
        user_id,
        tls: Default::default(),
        alpn: vec!["http/1.1".to_string()],
        auth: MsgraphAuthConfig { token },
    })
}

/// Prompts for an OAuth token command, falling back to an empty raw
/// token the user must fill in later.
fn oauth_token_secret() -> Result<Secret> {
    let command = prompt::some_text(
        "OAuth token command (an external token manager such as `ortie`); leave empty to set the token later:",
        None::<&str>,
    )?;

    let secret = match command {
        Some(command) if !command.trim().is_empty() => Secret::Command(shell(command.trim())),
        _ => Secret::Raw(String::new().into()),
    };

    Ok(secret)
}

/// Prints the OAuth notice: Himalaya does not mint or refresh tokens.
fn print_oauth_notice(label: &str, key: &str) {
    println!();
    println!("{label} uses OAuth 2.0. Himalaya does not manage OAuth tokens itself;");
    println!(
        "install a token manager (such as `ortie`) and point `{key}.auth.token.command` at it."
    );
    println!();
}

/// Classifies the provider from the email domain first (fast path for
/// consumer addresses), then from the autoconfig response (which has
/// already resolved custom Workspace / Microsoft 365 domains via MX).
fn detect_provider(domain: &str, autoconfig: Option<&Autoconfig>) -> Provider {
    if let Some(provider) = provider_from_domain(domain) {
        return provider;
    }

    autoconfig
        .map(provider_from_autoconfig)
        .unwrap_or(Provider::Other)
}

/// Recognizes the well-known consumer domains.
fn provider_from_domain(domain: &str) -> Option<Provider> {
    match domain.to_lowercase().as_str() {
        "gmail.com" | "googlemail.com" => Some(Provider::Google),
        "outlook.com" | "hotmail.com" | "live.com" | "msn.com" | "passport.com" => {
            Some(Provider::Microsoft)
        }
        _ => None,
    }
}

/// Recognizes the provider from the autoconfig id and server hostnames.
fn provider_from_autoconfig(autoconfig: &Autoconfig) -> Provider {
    let provider = &autoconfig.email_provider;
    let hosts = provider
        .incoming_server
        .iter()
        .chain(provider.outgoing_server.iter())
        .filter_map(|server| server.hostname.as_deref())
        .collect::<Vec<_>>()
        .join(" ");
    let haystack = format!("{} {hosts}", provider.id).to_lowercase();

    if haystack.contains("google") || haystack.contains("gmail") {
        Provider::Google
    } else if haystack.contains("outlook")
        || haystack.contains("office365")
        || haystack.contains("microsoft")
        || haystack.contains("hotmail")
    {
        Provider::Microsoft
    } else {
        Provider::Other
    }
}

/// Human-facing provider name for notices.
fn provider_label(provider: Provider) -> &'static str {
    match provider {
        Provider::Google => "Gmail",
        Provider::Microsoft => "Outlook",
        Provider::Other => "This provider",
    }
}

/// Pre-filled IMAP defaults for the well-known providers, so the user
/// does not have to type the host and port by hand.
fn provider_imap_default(provider: Provider, email: &str) -> Option<WizardImapConfig> {
    let (host, port) = match provider {
        Provider::Google => ("imap.gmail.com", 993),
        Provider::Microsoft => ("outlook.office365.com", 993),
        Provider::Other => return None,
    };

    Some(WizardImapConfig {
        host: host.to_string(),
        port,
        encryption: ImapEncryption::Tls,
        login: email.to_string(),
        auth: ImapAuth::Password(ImapSecret::Raw(String::new().into())),
    })
}

/// Pre-filled SMTP defaults for the well-known providers.
fn provider_smtp_default(provider: Provider, email: &str) -> Option<WizardSmtpConfig> {
    let (host, port, encryption) = match provider {
        Provider::Google => ("smtp.gmail.com", 465, SmtpEncryption::Tls),
        Provider::Microsoft => ("smtp.office365.com", 587, SmtpEncryption::StartTls),
        Provider::Other => return None,
    };

    Some(WizardSmtpConfig {
        host: host.to_string(),
        port,
        encryption,
        login: email.to_string(),
        auth: SmtpAuth::Password(SmtpSecret::Raw(String::new().into())),
    })
}

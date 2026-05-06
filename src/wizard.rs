//! Interactive configuration wizard.
//!
//! Triggered by `cli::load_or_wizard` when no config file is found
//! ([`pimalaya_config::toml::TomlConfig::from_paths_or_default`]
//! returned `Ok(None)`).
//!
//! Flow:
//!
//! 1. Confirm with the user. Exit if they decline.
//! 2. Ask for an account name and email address.
//! 3. Run discovery sequentially — PACC first, then Mozilla
//!    Autoconfig — with one spinner per method. Sub-step messages
//!    track which URL is being probed.
//! 4. If PACC returned a JMAP endpoint, ask the user whether to use
//!    it instead of IMAP+SMTP and run the matching protocol wizard(s).
//! 5. Build a [`Config`], write it to `target`, return it.

use std::{collections::HashMap, path::Path, process::exit, process::Command};

use anyhow::{anyhow, bail, Result};
use io_discovery::{
    autoconfig::{
        client::DiscoveryAutoconfigClient,
        coroutines::{dns_mx::mx_parent_domain, isp::DiscoveryIsp},
        types::{Autoconfig, SecurityType, Server, ServerType},
    },
    pacc::{
        client::{DiscoveryPaccClient, DiscoveryPaccClientError},
        types::PaccConfig,
    },
};
use log::{debug, info};
use pimalaya_cli::{
    prompt,
    spinner::Spinner,
    wizard::{
        imap::{
            self as imap_wizard, Encryption as ImapEncryption, ImapAuth, ImapSecret,
            WizardImapConfig,
        },
        jmap::{self as jmap_wizard, JmapAuth, JmapSecret, WizardJmapConfig},
        smtp::{
            self as smtp_wizard, Encryption as SmtpEncryption, SmtpAuth, SmtpSecret,
            WizardSmtpConfig,
        },
    },
};
use pimalaya_config::{command::shell, secret::Secret};
use url::Url;

use crate::config::{
    AccountConfig, Config, ImapConfig, JmapAuthConfig, JmapConfig, SaslConfig, SaslMechanismConfig,
    SaslPlainConfig, SmtpConfig,
};

/// DNS resolver used by PACC and Autoconfig discovery. Cloudflare's
/// `1.1.1.1` is a reasonable default; we'll make this configurable
/// later.
const DEFAULT_RESOLVER: &str = "tcp://1.1.1.1:53";

pub fn run_or_exit(target: &Path) -> Result<Config> {
    let prompt = format!(
        "No configuration found. Create one at {}?",
        target.display(),
    );

    if !prompt::bool(&prompt, true)? {
        exit(0);
    }

    let account_name = prompt::text("Account name:", Some("default"))?;
    let email = prompt::text::<&str>("Email address:", None)?;

    let (local_part, domain) = email
        .rsplit_once('@')
        .ok_or_else(|| anyhow!("Invalid email address `{email}`: missing `@`"))?;

    let discovery = discover(local_part, domain);

    let account = build_account_from_discovery(&account_name, local_part, domain, discovery)?;

    let config = Config {
        downloads_dir: None,
        table_preset: None,
        table_arrangement: None,
        envelope: Default::default(),
        message: Default::default(),
        accounts: HashMap::from([(account_name, account)]),
    };

    config.write(target)?;
    info!("Configuration written to {}.", target.display());

    Ok(config)
}

struct DiscoveryResult {
    imap: Option<WizardImapConfig>,
    smtp: Option<WizardSmtpConfig>,
    jmap: Option<WizardJmapConfig>,
}

/// Drives PACC then Mozilla Autoconfig sequentially, each with its
/// own spinner. PACC values win on overlap; Autoconfig only fills
/// IMAP/SMTP fields PACC didn't yield. JMAP is PACC-only.
fn discover(local_part: &str, domain: &str) -> DiscoveryResult {
    let pacc = run_pacc_with_spinner(domain);
    let autoconfig = run_autoconfig_with_spinner(local_part, domain);

    let (pacc_imap, pacc_smtp, pacc_jmap) = pacc
        .as_ref()
        .map(pacc_defaults)
        .unwrap_or((None, None, None));
    let (ac_imap, ac_smtp) = autoconfig
        .as_ref()
        .map(autoconfig_defaults)
        .unwrap_or((None, None));

    DiscoveryResult {
        imap: pacc_imap.or(ac_imap),
        smtp: pacc_smtp.or(ac_smtp),
        jmap: pacc_jmap,
    }
}

fn run_pacc_with_spinner(domain: &str) -> Option<PaccConfig> {
    let spinner = Spinner::start(format!("Probing PACC for {domain}…"));

    let resolver: Url = match DEFAULT_RESOLVER.parse() {
        Ok(url) => url,
        Err(err) => {
            debug!("PACC: invalid default resolver `{DEFAULT_RESOLVER}`: {err}");
            spinner.failure(format!("PACC: invalid resolver `{DEFAULT_RESOLVER}`"));
            return None;
        }
    };

    spinner.set_message(format!(
        "PACC: fetching .well-known config from ua-auto-config.{domain} and verifying digest…"
    ));

    let mut client = DiscoveryPaccClient::new(resolver);
    match client.discover(domain) {
        Ok(config) => {
            spinner.success(pacc_summary(domain, &config));
            Some(config)
        }
        Err(DiscoveryPaccClientError::Discovery(err)) => {
            debug!("PACC discovery for {domain} failed: {err}");
            spinner.failure(format!("PACC: no valid configuration for {domain}"));
            None
        }
        Err(err) => {
            debug!("PACC transport error for {domain}: {err}");
            spinner.failure(format!("PACC: endpoint unreachable for {domain}"));
            None
        }
    }
}

fn pacc_summary(domain: &str, config: &PaccConfig) -> String {
    let p = &config.protocols;
    let mut protos = Vec::with_capacity(3);
    if p.jmap.is_some() {
        protos.push("JMAP");
    }
    if p.imap.is_some() {
        protos.push("IMAP");
    }
    if p.smtp.is_some() {
        protos.push("SMTP");
    }
    if protos.is_empty() {
        format!("PACC: configuration found for {domain} (no IMAP/SMTP/JMAP fields)")
    } else {
        format!("PACC: discovered {} for {domain}", protos.join(" + "))
    }
}

/// Tries the Mozilla Autoconfig chain — direct ISP URLs, then
/// MX-derived parent domain ISP URLs. The TXT mailconf and SRV
/// fallbacks from the autoconfig CLI are skipped here; we keep the
/// wizard fast and let manual entry handle the long tail. The
/// spinner message is updated for every sub-attempt so the user sees
/// which URL is currently being probed.
fn run_autoconfig_with_spinner(local_part: &str, domain: &str) -> Option<Autoconfig> {
    let spinner = Spinner::start(format!("Probing Mozilla Autoconfig for {domain}…"));

    let resolver: Url = match DEFAULT_RESOLVER.parse() {
        Ok(url) => url,
        Err(err) => {
            debug!("Autoconfig: invalid default resolver `{DEFAULT_RESOLVER}`: {err}");
            spinner.failure(format!("Autoconfig: invalid resolver `{DEFAULT_RESOLVER}`"));
            return None;
        }
    };

    let mut client = DiscoveryAutoconfigClient::new(resolver);

    if let Some(ac) = try_isp_urls_with_spinner(&spinner, &mut client, local_part, domain) {
        spinner.success(autoconfig_summary(domain, &ac));
        return Some(ac);
    }

    spinner.set_message(format!("Autoconfig: looking up MX records for {domain}…"));

    let mx_parent = match client.mx(domain) {
        Ok(records) => records
            .first()
            .map(|r| r.rdata.exchange.to_string())
            .and_then(|t| mx_parent_domain(&t))
            .filter(|d| d != domain),
        Err(err) => {
            debug!("Autoconfig MX lookup for {domain} failed: {err}");
            None
        }
    };

    if let Some(parent) = mx_parent {
        debug!("Autoconfig: re-trying ISPs against MX parent {parent}");
        if let Some(ac) = try_isp_urls_with_spinner(&spinner, &mut client, local_part, &parent) {
            spinner.success(autoconfig_summary(domain, &ac));
            return Some(ac);
        }
    }

    spinner.failure(format!(
        "Autoconfig: no provider configuration found for {domain}"
    ));
    None
}

const ISP_LABELS: [&str; 5] = [
    "ISP main URL (HTTPS)",
    "ISP main URL (HTTP)",
    "ISP well-known URL (HTTPS)",
    "ISP well-known URL (HTTP)",
    "Thunderbird ISPDB",
];

fn try_isp_urls_with_spinner(
    spinner: &Spinner,
    client: &mut DiscoveryAutoconfigClient,
    local_part: &str,
    domain: &str,
) -> Option<Autoconfig> {
    let urls = match DiscoveryIsp::all_urls(local_part, domain) {
        Ok(urls) => urls,
        Err(err) => {
            debug!("Autoconfig: cannot build ISP URLs for {domain}: {err}");
            return None;
        }
    };

    for (url, label) in urls.iter().zip(ISP_LABELS.iter()) {
        spinner.set_message(format!("Autoconfig: trying {label} for {domain}…"));
        match client.isp(url.clone()) {
            Ok(ac) => return Some(ac),
            Err(err) => debug!("Autoconfig ISP attempt at {url} failed: {err}"),
        }
    }

    None
}

fn autoconfig_summary(domain: &str, ac: &Autoconfig) -> String {
    let has_imap = ac
        .email_provider
        .incoming_server
        .iter()
        .any(|s| matches!(s.r#type, ServerType::Imap));
    let has_smtp = ac
        .email_provider
        .outgoing_server
        .iter()
        .any(|s| matches!(s.r#type, ServerType::Smtp));
    let mut protos = Vec::with_capacity(2);
    if has_imap {
        protos.push("IMAP");
    }
    if has_smtp {
        protos.push("SMTP");
    }
    if protos.is_empty() {
        format!("Autoconfig: configuration found for {domain} (no IMAP/SMTP fields)")
    } else {
        format!("Autoconfig: discovered {} for {domain}", protos.join(" + "))
    }
}

fn autoconfig_defaults(ac: &Autoconfig) -> (Option<WizardImapConfig>, Option<WizardSmtpConfig>) {
    let imap = ac
        .email_provider
        .incoming_server
        .iter()
        .find(|s| matches!(s.r#type, ServerType::Imap))
        .and_then(autoconfig_imap);

    let smtp = ac
        .email_provider
        .outgoing_server
        .iter()
        .find(|s| matches!(s.r#type, ServerType::Smtp))
        .and_then(autoconfig_smtp);

    (imap, smtp)
}

fn autoconfig_imap(server: &Server) -> Option<WizardImapConfig> {
    let host = server.hostname.clone()?;
    let encryption = match server.socket_type {
        Some(SecurityType::Tls) => ImapEncryption::Tls,
        Some(SecurityType::Starttls) => ImapEncryption::StartTls,
        _ => ImapEncryption::None,
    };
    let port = server.port.unwrap_or(match encryption {
        ImapEncryption::Tls => 993,
        _ => 143,
    });

    Some(WizardImapConfig {
        host,
        port,
        encryption,
        login: String::new(),
        auth: ImapAuth::Password(ImapSecret::Raw(String::new().into())),
    })
}

fn autoconfig_smtp(server: &Server) -> Option<WizardSmtpConfig> {
    let host = server.hostname.clone()?;
    let encryption = match server.socket_type {
        Some(SecurityType::Tls) => SmtpEncryption::Tls,
        Some(SecurityType::Starttls) => SmtpEncryption::StartTls,
        _ => SmtpEncryption::None,
    };
    let port = server.port.unwrap_or(match encryption {
        SmtpEncryption::Tls => 465,
        SmtpEncryption::StartTls => 587,
        SmtpEncryption::None => 25,
    });

    Some(WizardSmtpConfig {
        host,
        port,
        encryption,
        login: String::new(),
        auth: SmtpAuth::Password(SmtpSecret::Raw(String::new().into())),
    })
}

fn pacc_defaults(
    config: &PaccConfig,
) -> (
    Option<WizardImapConfig>,
    Option<WizardSmtpConfig>,
    Option<WizardJmapConfig>,
) {
    let imap = config.protocols.imap.as_ref().map(|p| WizardImapConfig {
        host: p.host.clone(),
        port: 993,
        encryption: ImapEncryption::Tls,
        login: String::new(),
        // Placeholder; only host/port/encryption are read as defaults.
        // The user picks their real auth in the wizard.
        auth: ImapAuth::Password(ImapSecret::Raw(String::new().into())),
    });

    let smtp = config.protocols.smtp.as_ref().map(|p| WizardSmtpConfig {
        host: p.host.clone(),
        port: 465,
        encryption: SmtpEncryption::Tls,
        login: String::new(),
        auth: SmtpAuth::Password(SmtpSecret::Raw(String::new().into())),
    });

    let jmap = config.protocols.jmap.as_ref().map(|p| WizardJmapConfig {
        server: p.url.clone(),
        // Placeholder; auth is replaced by the user in the wizard.
        auth: JmapAuth::Basic {
            login: String::new(),
            secret: JmapSecret::Raw(String::new().into()),
        },
    });

    (imap, smtp, jmap)
}

/// Derives default [`WizardImapConfig`] values from an existing
/// [`ImapConfig`]. Used by `account configure` to pre-fill the wizard
/// prompts with the account's current values. The auth secret is
/// never reused — the wizard re-prompts the user for it.
pub(crate) fn imap_to_wizard(c: &ImapConfig) -> WizardImapConfig {
    let scheme = c.url.scheme();
    let encryption = match scheme {
        "imaps" => ImapEncryption::Tls,
        _ if c.starttls => ImapEncryption::StartTls,
        _ => ImapEncryption::None,
    };
    let host = c.url.host_str().unwrap_or("").to_string();
    let port = c.url.port_or_known_default().unwrap_or(match encryption {
        ImapEncryption::Tls => 993,
        _ => 143,
    });
    let login = c
        .sasl
        .plain
        .as_ref()
        .map(|p| p.authcid.clone())
        .or_else(|| c.sasl.login.as_ref().map(|l| l.username.clone()))
        .unwrap_or_default();

    WizardImapConfig {
        host,
        port,
        encryption,
        login,
        auth: ImapAuth::Password(ImapSecret::Raw(String::new().into())),
    }
}

/// Same as [`imap_to_wizard`] but for SMTP.
pub(crate) fn smtp_to_wizard(c: &SmtpConfig) -> WizardSmtpConfig {
    let scheme = c.url.scheme();
    let encryption = match scheme {
        "smtps" => SmtpEncryption::Tls,
        _ if c.starttls => SmtpEncryption::StartTls,
        _ => SmtpEncryption::None,
    };
    let host = c.url.host_str().unwrap_or("").to_string();
    let port = c.url.port_or_known_default().unwrap_or(match encryption {
        SmtpEncryption::Tls => 465,
        SmtpEncryption::StartTls => 587,
        SmtpEncryption::None => 25,
    });
    let login = c
        .sasl
        .plain
        .as_ref()
        .map(|p| p.authcid.clone())
        .or_else(|| c.sasl.login.as_ref().map(|l| l.username.clone()))
        .unwrap_or_default();

    WizardSmtpConfig {
        host,
        port,
        encryption,
        login,
        auth: SmtpAuth::Password(SmtpSecret::Raw(String::new().into())),
    }
}

/// Same as [`imap_to_wizard`] but for JMAP. Auth is reset to a
/// placeholder — the wizard re-prompts the user for it.
pub(crate) fn jmap_to_wizard(c: &JmapConfig) -> WizardJmapConfig {
    let auth = match &c.auth {
        JmapAuthConfig::Basic { username, .. } => JmapAuth::Basic {
            login: username.clone(),
            secret: JmapSecret::Raw(String::new().into()),
        },
        JmapAuthConfig::Bearer { .. } | JmapAuthConfig::Header(_) => JmapAuth::Bearer {
            secret: JmapSecret::Raw(String::new().into()),
        },
    };

    WizardJmapConfig {
        server: c.server.clone(),
        auth,
    }
}

/// Decides whether to run the JMAP wizard or the IMAP+SMTP wizard
/// pair and builds an [`AccountConfig`] from the answers. The JMAP
/// branch fires when PACC discovered a JMAP endpoint and either the
/// user opted into it (when IMAP+SMTP defaults were also present) or
/// nothing else is available.
fn build_account_from_discovery(
    account_name: &str,
    local_part: &str,
    domain: &str,
    discovery: DiscoveryResult,
) -> Result<AccountConfig> {
    let DiscoveryResult { imap, smtp, jmap } = discovery;

    let prefer_jmap = match (&jmap, imap.is_some() || smtp.is_some()) {
        (Some(_), true) => prompt::bool(
            "A JMAP server was discovered. Use it instead of IMAP+SMTP?",
            true,
        )?,
        (Some(_), false) => true,
        (None, _) => false,
    };

    if prefer_jmap {
        let jmap_defaults = jmap.as_ref();
        let jmap = jmap_wizard::run(account_name, local_part, domain, jmap_defaults)?;

        Ok(AccountConfig {
            default: true,
            downloads_dir: None,
            table_preset: None,
            table_arrangement: None,
            envelope: Default::default(),
            imap: None,
            jmap: Some(jmap_to_config(jmap)?),
            maildir: None,
            smtp: None,
        })
    } else {
        let imap = imap_wizard::run(account_name, local_part, domain, imap.as_ref())?;
        let smtp = smtp_wizard::run(account_name, local_part, domain, smtp.as_ref())?;

        Ok(AccountConfig {
            default: true,
            downloads_dir: None,
            table_preset: None,
            table_arrangement: None,
            envelope: Default::default(),
            imap: Some(imap_to_config(imap)?),
            jmap: None,
            maildir: None,
            smtp: Some(smtp_to_config(smtp)?),
        })
    }
}

/// Edits (or creates) the account named `account_name`. Uses the
/// account's current `jmap` or `imap`/`smtp` blocks as defaults; an
/// existing JMAP block routes to the JMAP wizard, otherwise the
/// IMAP+SMTP wizards run. Skips provider discovery entirely — this is
/// meant for accounts the user already configured. Writes the
/// updated config to `target` before returning.
pub fn edit_account(target: &Path, mut config: Config, account_name: &str) -> Result<Config> {
    let existing = config.accounts.remove(account_name);

    let jmap_defaults = existing
        .as_ref()
        .and_then(|a| a.jmap.as_ref())
        .map(jmap_to_wizard);
    let imap_defaults = existing
        .as_ref()
        .and_then(|a| a.imap.as_ref())
        .map(imap_to_wizard);
    let smtp_defaults = existing
        .as_ref()
        .and_then(|a| a.smtp.as_ref())
        .map(smtp_to_wizard);

    let default_email = imap_defaults
        .as_ref()
        .map(|c| c.login.clone())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            smtp_defaults
                .as_ref()
                .map(|c| c.login.clone())
                .filter(|s| !s.is_empty())
        })
        .or_else(|| match jmap_defaults.as_ref().map(|c| &c.auth) {
            Some(JmapAuth::Basic { login, .. }) if !login.is_empty() => Some(login.clone()),
            _ => None,
        });

    let email = prompt::text("Email address:", default_email.as_deref())?;
    let (local_part, domain) = email
        .rsplit_once('@')
        .ok_or_else(|| anyhow!("Invalid email address `{email}`: missing `@`"))?;

    let is_first_account = config.accounts.is_empty() && existing.is_none();
    let default = existing
        .as_ref()
        .map(|a| a.default)
        .unwrap_or(is_first_account);
    let downloads_dir = existing.as_ref().and_then(|a| a.downloads_dir.clone());
    let table_preset = existing.as_ref().and_then(|a| a.table_preset.clone());
    let table_arrangement = existing.as_ref().and_then(|a| a.table_arrangement.clone());
    let envelope = existing
        .as_ref()
        .map(|a| a.envelope.clone())
        .unwrap_or_default();
    let maildir = existing.as_ref().and_then(|a| a.maildir.clone());

    let account = if jmap_defaults.is_some() {
        let jmap = jmap_wizard::run(account_name, local_part, domain, jmap_defaults.as_ref())?;
        AccountConfig {
            default,
            downloads_dir,
            table_preset,
            table_arrangement,
            envelope,
            imap: None,
            jmap: Some(jmap_to_config(jmap)?),
            maildir,
            smtp: None,
        }
    } else {
        let imap = imap_wizard::run(account_name, local_part, domain, imap_defaults.as_ref())?;
        let smtp = smtp_wizard::run(account_name, local_part, domain, smtp_defaults.as_ref())?;
        AccountConfig {
            default,
            downloads_dir,
            table_preset,
            table_arrangement,
            envelope,
            imap: Some(imap_to_config(imap)?),
            jmap: None,
            maildir,
            smtp: Some(smtp_to_config(smtp)?),
        }
    };

    config.accounts.insert(account_name.to_owned(), account);
    config.write(target)?;
    info!("Configuration written to {}.", target.display());

    Ok(config)
}

fn imap_to_config(w: WizardImapConfig) -> Result<ImapConfig> {
    let scheme = match w.encryption {
        ImapEncryption::Tls => "imaps",
        ImapEncryption::StartTls | ImapEncryption::None => "imap",
    };
    let url = Url::parse(&format!("{scheme}://{}:{}", w.host, w.port))?;
    let starttls = matches!(w.encryption, ImapEncryption::StartTls);
    let sasl = build_sasl_imap(&w.login, w.auth)?;

    Ok(ImapConfig {
        url,
        tls: Default::default(),
        starttls,
        sasl,
    })
}

fn smtp_to_config(w: WizardSmtpConfig) -> Result<SmtpConfig> {
    let scheme = match w.encryption {
        SmtpEncryption::Tls => "smtps",
        SmtpEncryption::StartTls | SmtpEncryption::None => "smtp",
    };
    let url = Url::parse(&format!("{scheme}://{}:{}", w.host, w.port))?;
    let starttls = matches!(w.encryption, SmtpEncryption::StartTls);
    let sasl = build_sasl_smtp(&w.login, w.auth)?;

    Ok(SmtpConfig {
        url,
        tls: Default::default(),
        starttls,
        sasl,
    })
}

fn jmap_to_config(w: WizardJmapConfig) -> Result<JmapConfig> {
    let auth = match w.auth {
        JmapAuth::Basic { login, secret } => JmapAuthConfig::Basic {
            username: login,
            password: jmap_secret_to_secret(secret)?,
        },
        JmapAuth::Bearer { secret } => JmapAuthConfig::Bearer {
            token: jmap_secret_to_secret(secret)?,
        },
    };

    Ok(JmapConfig {
        server: w.server,
        tls: Default::default(),
        auth,
        identity_id: None,
        drafts_mailbox_id: None,
    })
}

fn build_sasl_imap(login: &str, auth: ImapAuth) -> Result<SaslConfig> {
    let ImapAuth::Password(secret) = auth;
    let passwd = match secret {
        ImapSecret::Raw(s) => Secret::Raw(s),
        ImapSecret::Command(cmd) => Secret::Command(parse_cmd(&cmd)?),
    };

    Ok(plain_sasl(login, passwd))
}

fn build_sasl_smtp(login: &str, auth: SmtpAuth) -> Result<SaslConfig> {
    let SmtpAuth::Password(secret) = auth;
    let passwd = match secret {
        SmtpSecret::Raw(s) => Secret::Raw(s),
        SmtpSecret::Command(cmd) => Secret::Command(parse_cmd(&cmd)?),
    };

    Ok(plain_sasl(login, passwd))
}

fn jmap_secret_to_secret(secret: JmapSecret) -> Result<Secret> {
    Ok(match secret {
        JmapSecret::Raw(s) => Secret::Raw(s),
        JmapSecret::Command(cmd) => Secret::Command(parse_cmd(&cmd)?),
    })
}

fn plain_sasl(login: &str, passwd: Secret) -> SaslConfig {
    SaslConfig {
        mechanism: Some(SaslMechanismConfig::Plain),
        login: None,
        plain: Some(SaslPlainConfig {
            authzid: None,
            authcid: login.to_owned(),
            passwd,
        }),
        anonymous: None,
    }
}

fn parse_cmd(cmd: &str) -> Result<Command> {
    let line = cmd.trim();
    if line.is_empty() {
        bail!("Empty shell command for secret");
    }
    Ok(shell(line))
}

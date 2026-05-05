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
//! 3. Run discovery — currently PACC only; Mozilla Autoconfig will
//!    join it once `io-discovery`'s `autoconfig` feature builds again.
//!    Both probes will then run in parallel via `std::thread::scope`,
//!    PACC results preferred.
//! 4. Convert any discovery hit into [`WizardImapConfig`] /
//!    [`WizardSmtpConfig`] defaults, hand them to the per-protocol
//!    wizards in [`pimalaya_cli::wizard`].
//! 5. Build a [`Config`], write it to `target`, return it.

use std::{collections::HashMap, path::Path, process::exit, thread};

use anyhow::{anyhow, bail, Result};
use io_discovery::pacc::{
    client::{DiscoveryPaccClient, DiscoveryPaccClientError},
    types::PaccConfig,
};
use io_process::command::Command;
use log::{debug, info};
use pimalaya_cli::wizard::{
    imap::{
        self as imap_wizard, Encryption as ImapEncryption, ImapAuth, ImapSecret, WizardImapConfig,
    },
    smtp::{
        self as smtp_wizard, Encryption as SmtpEncryption, SmtpAuth, SmtpSecret, WizardSmtpConfig,
    },
};
use pimalaya_config::secret::Secret;
use url::Url;

use crate::config::{
    AccountConfig, Config, ImapConfig, SaslConfig, SaslMechanismConfig, SaslPlainConfig, SmtpConfig,
};

/// DNS resolver used by PACC discovery. Cloudflare's `1.1.1.1` is a
/// reasonable default; we'll make this configurable later.
const DEFAULT_RESOLVER: &str = "tcp://1.1.1.1:53";

pub fn run_or_exit(target: &Path) -> Result<Config> {
    let prompt = format!(
        "No configuration found. Create one at {}?",
        target.display(),
    );

    if !pimalaya_cli::prompt::bool(&prompt, true)? {
        exit(0);
    }

    let account_name = pimalaya_cli::prompt::text("Account name:", Some("default"))?;
    let email = pimalaya_cli::prompt::text::<&str>("Email address:", None)?;

    let (local_part, domain) = email
        .split_once('@')
        .ok_or_else(|| anyhow!("Invalid email address `{email}`: missing `@`"))?;

    info!("Discovering provider settings for {domain}…");
    let (imap_defaults, smtp_defaults) = discover(domain);

    let imap = imap_wizard::run(&account_name, local_part, domain, imap_defaults.as_ref())?;
    let smtp = smtp_wizard::run(&account_name, local_part, domain, smtp_defaults.as_ref())?;

    let account = AccountConfig {
        default: true,
        downloads_dir: None,
        table_preset: None,
        table_arrangement: None,
        imap: Some(imap_to_config(imap)?),
        jmap: None,
        maildir: None,
        smtp: Some(smtp_to_config(smtp)?),
    };

    let config = Config {
        downloads_dir: None,
        table_preset: None,
        table_arrangement: None,
        accounts: HashMap::from([(account_name, account)]),
    };

    config.write(target)?;
    info!("Configuration written to {}.", target.display());

    Ok(config)
}

/// Runs configured discovery probes in parallel and returns the
/// merged IMAP/SMTP defaults. Currently PACC-only; Mozilla Autoconfig
/// will join once io-discovery's `autoconfig` feature compiles.
fn discover(domain: &str) -> (Option<WizardImapConfig>, Option<WizardSmtpConfig>) {
    thread::scope(|scope| {
        let pacc = scope.spawn(|| run_pacc(domain));

        let pacc = pacc.join().unwrap_or_else(|_| {
            debug!("PACC discovery thread panicked");
            None
        });

        match pacc {
            Some(config) => pacc_defaults(&config),
            None => (None, None),
        }
    })
}

fn run_pacc(domain: &str) -> Option<PaccConfig> {
    let resolver: Url = match DEFAULT_RESOLVER.parse() {
        Ok(url) => url,
        Err(err) => {
            debug!("PACC: invalid default resolver `{DEFAULT_RESOLVER}`: {err}");
            return None;
        }
    };

    let mut client = DiscoveryPaccClient::new(resolver);
    match client.discover(domain) {
        Ok(config) => Some(config),
        Err(DiscoveryPaccClientError::Discovery(err)) => {
            debug!("PACC discovery for {domain} failed: {err}");
            None
        }
        Err(err) => {
            debug!("PACC transport error for {domain}: {err}");
            None
        }
    }
}

fn pacc_defaults(config: &PaccConfig) -> (Option<WizardImapConfig>, Option<WizardSmtpConfig>) {
    let imap = config.protocols.imap.as_ref().map(|p| WizardImapConfig {
        host: p.host.clone(),
        port: 993,
        encryption: ImapEncryption::Tls,
        login: String::new(),
        // Placeholder; the user picks their real auth in the wizard.
        // Only the host/port/encryption fields are read as defaults.
        auth: ImapAuth::Password(ImapSecret::Raw(String::new().into())),
    });

    let smtp = config.protocols.smtp.as_ref().map(|p| WizardSmtpConfig {
        host: p.host.clone(),
        port: 465,
        encryption: SmtpEncryption::Tls,
        login: String::new(),
        auth: SmtpAuth::Password(SmtpSecret::Raw(String::new().into())),
    });

    (imap, smtp)
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
    if cmd.trim().is_empty() {
        bail!("Empty shell command for secret");
    }
    Ok(Command::new(cmd))
}

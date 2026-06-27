//! Interactive configuration wizard for editing (or creating) an
//! existing account. Skips provider discovery entirely: this is meant
//! for accounts the user already configured. Pre-fills the wizard
//! prompts with the account's current values; the auth secret is
//! never reused, the user is re-prompted for it.

use std::path::Path;

use anyhow::{Result, anyhow};
use log::info;
use pimalaya_cli::{
    prompt,
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

use crate::{
    config::{
        AccountConfig, Config, ImapConfig, JmapAuthConfig, JmapConfig, SaslConfig, SmtpConfig,
    },
    wizard::account::{imap_to_config, jmap_to_config, smtp_to_config},
};

/// Edits (or creates) the account named `account_name`. Uses the
/// account's current `jmap` or `imap`/`smtp` blocks as defaults; an
/// existing JMAP block routes to the JMAP wizard, otherwise the
/// IMAP+SMTP wizards run. Writes the updated config to `target`
/// before returning.
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
    let table = existing
        .as_ref()
        .map(|a| a.table.clone())
        .unwrap_or_default();
    let envelope = existing
        .as_ref()
        .map(|a| a.envelope.clone())
        .unwrap_or_default();
    let mailbox = existing
        .as_ref()
        .map(|a| a.mailbox.clone())
        .unwrap_or_default();
    let attachment = existing
        .as_ref()
        .map(|a| a.attachment.clone())
        .unwrap_or_default();
    let maildir = existing.as_ref().and_then(|a| a.maildir.clone());
    let m2dir = existing.as_ref().and_then(|a| a.m2dir.clone());

    let account = if jmap_defaults.is_some() {
        let jmap = jmap_wizard::run(account_name, local_part, domain, jmap_defaults.as_ref())?;
        AccountConfig {
            default,
            downloads_dir,
            table,
            envelope,
            mailbox,
            attachment,
            imap: None,
            jmap: Some(jmap_to_config(jmap)?),
            gmail: None,
            msgraph: None,
            maildir,
            m2dir,
            smtp: None,
        }
    } else {
        let imap = imap_wizard::run(account_name, local_part, domain, imap_defaults.as_ref())?;
        let smtp = smtp_wizard::run(account_name, local_part, domain, smtp_defaults.as_ref())?;
        AccountConfig {
            default,
            downloads_dir,
            table,
            envelope,
            mailbox,
            attachment,
            imap: Some(imap_to_config(imap)?),
            jmap: None,
            gmail: None,
            msgraph: None,
            maildir,
            m2dir,
            smtp: Some(smtp_to_config(smtp)?),
        }
    };

    config.accounts.insert(account_name.to_owned(), account);
    config.write(target)?;
    info!("Configuration written to {}.", target.display());

    Ok(config)
}

/// Derives default [`WizardImapConfig`] values from an existing
/// [`ImapConfig`]. The auth secret is never reused; the wizard
/// re-prompts the user for it.
pub fn imap_to_wizard(c: &ImapConfig) -> WizardImapConfig {
    let url = crate::imap::client::parse_imap_server(&c.server).ok();
    let scheme = url.as_ref().map(|u| u.scheme()).unwrap_or("imaps");
    let encryption = match scheme {
        "imaps" => ImapEncryption::Tls,
        _ if c.starttls => ImapEncryption::StartTls,
        _ => ImapEncryption::None,
    };
    let host = url
        .as_ref()
        .and_then(|u| u.host_str().map(str::to_owned))
        .unwrap_or_default();
    let port = url
        .as_ref()
        .and_then(|u| u.port_or_known_default())
        .unwrap_or(match encryption {
            ImapEncryption::Tls => 993,
            _ => 143,
        });
    let login = sasl_login(c.sasl.as_ref());

    WizardImapConfig {
        host,
        port,
        encryption,
        login,
        auth: ImapAuth::Password(ImapSecret::Raw(String::new().into())),
    }
}

/// Same as [`imap_to_wizard`] but for SMTP.
pub fn smtp_to_wizard(c: &SmtpConfig) -> WizardSmtpConfig {
    let url = crate::smtp::client::parse_smtp_server(&c.server).ok();
    let scheme = url.as_ref().map(|u| u.scheme()).unwrap_or("smtps");
    let encryption = match scheme {
        "smtps" => SmtpEncryption::Tls,
        _ if c.starttls => SmtpEncryption::StartTls,
        _ => SmtpEncryption::None,
    };
    let host = url
        .as_ref()
        .and_then(|u| u.host_str().map(str::to_owned))
        .unwrap_or_default();
    let port = url
        .as_ref()
        .and_then(|u| u.port_or_known_default())
        .unwrap_or(match encryption {
            SmtpEncryption::Tls => 465,
            SmtpEncryption::StartTls => 587,
            SmtpEncryption::None => 25,
        });
    let login = sasl_login(c.sasl.as_ref());

    WizardSmtpConfig {
        host,
        port,
        encryption,
        login,
        auth: SmtpAuth::Password(SmtpSecret::Raw(String::new().into())),
    }
}

/// Extracts the user-facing login (PLAIN authcid, LOGIN username,
/// XOAUTH2/OAUTHBEARER/SCRAM username) from a SASL block so the
/// wizard can pre-fill the prompt when editing an existing account.
/// Returns an empty string when the block is absent or carries no
/// username (e.g. ANONYMOUS).
fn sasl_login(sasl: Option<&SaslConfig>) -> String {
    match sasl {
        Some(SaslConfig::Plain(p)) => p.authcid.clone(),
        Some(SaslConfig::Login(l)) => l.username.clone(),
        Some(SaslConfig::Oauthbearer(o)) => o.username.clone(),
        Some(SaslConfig::Xoauth2(x)) => x.username.clone(),
        Some(SaslConfig::ScramSha256(s)) => s.username.clone(),
        Some(SaslConfig::Anonymous(_)) | None => String::new(),
    }
}

/// Same as [`imap_to_wizard`] but for JMAP. Auth is reset to a
/// placeholder; the wizard re-prompts the user for it.
pub fn jmap_to_wizard(c: &JmapConfig) -> WizardJmapConfig {
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

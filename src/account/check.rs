use std::{fmt, path::PathBuf};

use anyhow::{bail, Result};
use clap::Parser;
use pimalaya_cli::printer::Printer;
use pimalaya_config::toml::TomlConfig;
use serde::Serialize;

use crate::{
    cli::BackendFlag,
    config::{AccountConfig, Config},
};

/// Validate the account configuration.
///
/// Loads the TOML configuration, picks the active account (via the
/// global `--account` flag or the default), and checks each backend
/// allowed by `--backend`. The check tries to instantiate a client per
/// backend, which exercises the same handshake / authentication paths
/// the other commands would take.
#[derive(Debug, Parser)]
pub struct AccountCheckCommand;

impl AccountCheckCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config_paths: &[PathBuf],
        account_name: Option<&str>,
        backend: BackendFlag,
    ) -> Result<()> {
        let mut config = match Config::from_paths_or_default(config_paths)? {
            Some(config) => config,
            None => bail!(
                "No configuration found. Run `himalaya` once to launch the wizard, \
                 or `himalaya account configure <name>` to create one."
            ),
        };

        let (name, account_config) = config
            .take_account(account_name)?
            .ok_or_else(|| anyhow::anyhow!("Cannot find account"))?;

        let mut report = CheckReport {
            account: name,
            backends: Vec::new(),
        };

        #[cfg(feature = "imap")]
        if backend.allows_imap() {
            if let Some(imap_config) = account_config.imap.clone() {
                report
                    .backends
                    .push(check_imap(&config, &account_config, imap_config));
            }
        }

        #[cfg(feature = "jmap")]
        if backend.allows_jmap() {
            if let Some(jmap_config) = account_config.jmap.clone() {
                report
                    .backends
                    .push(check_jmap(&config, &account_config, jmap_config));
            }
        }

        #[cfg(feature = "maildir")]
        if backend.allows_maildir() {
            if let Some(maildir_config) = account_config.maildir.clone() {
                report
                    .backends
                    .push(check_maildir(&config, &account_config, maildir_config));
            }
        }

        #[cfg(feature = "smtp")]
        if backend.allows_smtp() {
            if let Some(smtp_config) = account_config.smtp.clone() {
                report
                    .backends
                    .push(check_smtp(&config, &account_config, smtp_config));
            }
        }

        if report.backends.is_empty() {
            bail!("No backend matching `{backend}` is configured for this account");
        }

        printer.out(report)
    }
}

#[cfg(feature = "imap")]
fn check_imap(
    _config: &Config,
    _account_config: &AccountConfig,
    imap_config: crate::config::ImapConfig,
) -> BackendCheck {
    use crate::imap::session::ImapSession;

    let result = (|| -> Result<()> {
        let _session = ImapSession::new(
            imap_config.url.clone(),
            imap_config.tls.clone().try_into()?,
            imap_config.starttls,
            imap_config.sasl.clone().try_into()?,
        )?;
        Ok(())
    })();

    BackendCheck::from("imap", result)
}

#[cfg(feature = "jmap")]
fn check_jmap(
    _config: &Config,
    _account_config: &AccountConfig,
    jmap_config: crate::config::JmapConfig,
) -> BackendCheck {
    use crate::jmap::session::JmapSession;

    let result = (|| -> Result<()> {
        let _session = JmapSession::new(
            jmap_config.server.clone(),
            jmap_config.tls.clone().try_into()?,
            jmap_config.auth.clone().try_into()?,
        )?;
        Ok(())
    })();

    BackendCheck::from("jmap", result)
}

#[cfg(feature = "maildir")]
fn check_maildir(
    _config: &Config,
    _account_config: &AccountConfig,
    maildir_config: crate::config::MaildirConfig,
) -> BackendCheck {
    let result = (|| -> Result<()> {
        if !maildir_config.root.is_dir() {
            bail!(
                "Maildir root `{}` does not exist or is not a directory",
                maildir_config.root.display()
            );
        }
        Ok(())
    })();

    BackendCheck::from("maildir", result)
}

#[cfg(feature = "smtp")]
fn check_smtp(
    _config: &Config,
    _account_config: &AccountConfig,
    smtp_config: crate::config::SmtpConfig,
) -> BackendCheck {
    use crate::smtp::session::SmtpSession;

    let result = (|| -> Result<()> {
        let _session = SmtpSession::new(
            smtp_config.url.clone(),
            smtp_config.tls.clone().try_into()?,
            smtp_config.starttls,
            smtp_config.sasl.clone().try_into()?,
        )?;
        Ok(())
    })();

    BackendCheck::from("smtp", result)
}

#[derive(Clone, Debug, Serialize)]
pub struct CheckReport {
    pub account: String,
    pub backends: Vec<BackendCheck>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BackendCheck {
    pub backend: &'static str,
    pub ok: bool,
    pub error: Option<String>,
}

impl BackendCheck {
    fn from(backend: &'static str, result: Result<()>) -> Self {
        match result {
            Ok(()) => Self {
                backend,
                ok: true,
                error: None,
            },
            Err(err) => Self {
                backend,
                ok: false,
                error: Some(format!("{err:#}")),
            },
        }
    }
}

impl fmt::Display for CheckReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Account: {}", self.account)?;
        for check in &self.backends {
            match &check.error {
                None => writeln!(f, "  {}: OK", check.backend)?,
                Some(err) => writeln!(f, "  {}: FAIL ({err})", check.backend)?,
            }
        }
        Ok(())
    }
}

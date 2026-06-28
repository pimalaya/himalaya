use std::{fmt, path::PathBuf};

use anyhow::{Result, bail};
use clap::Parser;
use pimalaya_cli::printer::Printer;
use pimalaya_config::toml::TomlConfig;
use serde::Serialize;

use crate::{
    backend::Backend,
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
        backend: Backend,
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

        #[cfg(feature = "gmail")]
        if backend.allows_gmail() {
            if let Some(gmail_config) = account_config.gmail.clone() {
                report
                    .backends
                    .push(check_gmail(&config, &account_config, gmail_config));
            }
        }

        #[cfg(feature = "msgraph")]
        if backend.allows_msgraph() {
            if let Some(msgraph_config) = account_config.msgraph.clone() {
                report
                    .backends
                    .push(check_msgraph(&config, &account_config, msgraph_config));
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

        #[cfg(feature = "m2dir")]
        if backend.allows_m2dir() {
            if let Some(m2dir_config) = account_config.m2dir.clone() {
                report
                    .backends
                    .push(check_m2dir(&config, &account_config, m2dir_config));
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
    use io_imap::client::ImapClientStd;
    use pimalaya_stream::sasl::Sasl;

    use crate::imap::id::resolve_auto_id_params;

    let result = (|| -> Result<()> {
        let tls = imap_config.tls.clone().into_tls(imap_config.alpn.clone());
        let auto_id = resolve_auto_id_params(&imap_config.id)?;
        let server = crate::imap::client::parse_imap_server(&imap_config.server)?;
        let sasl: Option<Sasl> = imap_config
            .sasl
            .clone()
            .and_then(|cfg| {
                let host = server.host_str()?;
                let port = server.port_or_known_default()?;
                Some(cfg.try_into_sasl(host, port))
            })
            .transpose()?;
        let _ = ImapClientStd::connect(&server, &tls, imap_config.starttls, sasl, auto_id)?;
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
    use io_jmap::client::JmapClientStd;

    use crate::jmap::client::{jmap_http_auth, parse_server_url};

    let result = (|| -> Result<()> {
        let tls = jmap_config.tls.clone().into_tls(jmap_config.alpn.clone());
        let http_auth = jmap_http_auth(jmap_config.auth.clone())?;
        let url = parse_server_url(&jmap_config.server)?;
        let mut client = JmapClientStd::connect(&url, &tls, http_auth)?;
        client.session_get(&url)?;
        Ok(())
    })();

    BackendCheck::from("jmap", result)
}

#[cfg(feature = "gmail")]
fn check_gmail(
    _config: &Config,
    _account_config: &AccountConfig,
    gmail_config: crate::config::GmailConfig,
) -> BackendCheck {
    use io_gmail::v1::client::{GmailClientStd, GmailClientStdConnectOptions};
    use secrecy::ExposeSecret;

    use crate::gmail::client::gmail_token;

    let result = (|| -> Result<()> {
        let tls = gmail_config.tls.clone().into_tls(gmail_config.alpn.clone());
        let token = gmail_token(gmail_config.auth.clone())?;
        let options = GmailClientStdConnectOptions {
            tls,
            user_id: gmail_config.user_id.clone(),
        };
        let mut client = GmailClientStd::connect(token.expose_secret(), options)?;
        client.profile_get()?;
        Ok(())
    })();

    BackendCheck::from("gmail", result)
}

#[cfg(feature = "msgraph")]
fn check_msgraph(
    _config: &Config,
    _account_config: &AccountConfig,
    msgraph_config: crate::config::MsgraphConfig,
) -> BackendCheck {
    use io_msgraph::v1::client::{MsgraphClientStd, MsgraphClientStdConnectOptions};
    use secrecy::ExposeSecret;

    use crate::msgraph::client::msgraph_token;

    let result = (|| -> Result<()> {
        let tls = msgraph_config
            .tls
            .clone()
            .into_tls(msgraph_config.alpn.clone());
        let token = msgraph_token(msgraph_config.auth.clone())?;
        let options = MsgraphClientStdConnectOptions {
            tls,
            user_id: msgraph_config.user_id.clone(),
        };
        let mut client = MsgraphClientStd::connect(token.expose_secret(), options)?;
        client.me()?;
        Ok(())
    })();

    BackendCheck::from("msgraph", result)
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

#[cfg(feature = "m2dir")]
fn check_m2dir(
    _config: &Config,
    _account_config: &AccountConfig,
    m2dir_config: crate::config::M2dirConfig,
) -> BackendCheck {
    let result = (|| -> Result<()> {
        if !m2dir_config.root.is_dir() {
            bail!(
                "m2dir root `{}` does not exist or is not a directory",
                m2dir_config.root.display()
            );
        }
        Ok(())
    })();

    BackendCheck::from("m2dir", result)
}

#[cfg(feature = "smtp")]
fn check_smtp(
    _config: &Config,
    _account_config: &AccountConfig,
    smtp_config: crate::config::SmtpConfig,
) -> BackendCheck {
    use std::net::Ipv4Addr;

    use io_smtp::{client::SmtpClientStd, rfc5321::types::ehlo_domain::EhloDomain};
    use pimalaya_stream::sasl::Sasl;

    let result = (|| -> Result<()> {
        let tls = smtp_config.tls.clone().into_tls(smtp_config.alpn.clone());
        let domain: EhloDomain<'static> = Ipv4Addr::new(127, 0, 0, 1).into();
        let server = crate::smtp::client::parse_smtp_server(&smtp_config.server)?;
        let sasl: Option<Sasl> = smtp_config
            .sasl
            .clone()
            .and_then(|cfg| {
                let host = server.host_str()?;
                let port = server.port_or_known_default()?;
                Some(cfg.try_into_sasl(host, port))
            })
            .transpose()?;
        let _client = SmtpClientStd::connect(&server, &tls, smtp_config.starttls, domain, sasl)?;
        Ok(())
    })();

    BackendCheck::from("smtp", result)
}

/// Aggregated account check result: one outcome per backend.
#[derive(Clone, Debug, Serialize)]
pub struct CheckReport {
    pub account: String,
    pub backends: Vec<BackendCheck>,
}

/// Outcome of checking a single backend's connection.
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

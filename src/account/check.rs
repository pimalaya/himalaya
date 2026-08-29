//! # Account check
//!
//! The `account check` command, opening one connection per configured
//! backend so a credential or an endpoint fails here rather than in the
//! middle of a real command.

use std::{fmt, path::PathBuf};

use anyhow::{Result, bail};
use clap::Parser;
#[cfg(feature = "imap")]
use io_sasl::mechanism::SaslMechanism;
use pimalaya_cli::printer::Printer;
use pimalaya_config::toml::TomlConfig;
use schemars::JsonSchema;
use serde::Serialize;

#[cfg(feature = "imap")]
use crate::config::ImapConfig;
use crate::{
    backend::Backend,
    config::{AccountConfig, Config},
};

/// Validate the account configuration.
///
/// Every backend `--backend` allows is opened, which exercises the same
/// handshake and authentication paths a real command takes. The report
/// names each backend and whether it answered.
#[derive(Debug, Parser)]
pub struct AccountCheckCommand;

impl AccountCheckCommand {
    /// Checks every allowed backend of the account and prints the report.
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
                "No configuration found. Run bare `himalaya` to launch the wizard \
                 and generate one."
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
        if backend.allows_imap()
            && let Some(imap_config) = &account_config.imap
        {
            report
                .backends
                .push(BackendCheck::from("imap", connect_imap(imap_config)));
        }

        #[cfg(feature = "jmap")]
        if backend.allows_jmap()
            && let Some(jmap_config) = &account_config.jmap
        {
            report
                .backends
                .push(BackendCheck::from("jmap", connect_jmap(jmap_config)));
        }

        #[cfg(feature = "gmail")]
        if backend.allows_gmail()
            && let Some(gmail_config) = &account_config.gmail
        {
            report
                .backends
                .push(BackendCheck::from("gmail", connect_gmail(gmail_config)));
        }

        #[cfg(feature = "msgraph")]
        if backend.allows_msgraph()
            && let Some(msgraph_config) = &account_config.msgraph
        {
            report.backends.push(BackendCheck::from(
                "msgraph",
                connect_msgraph(msgraph_config),
            ));
        }

        #[cfg(feature = "maildir")]
        if backend.allows_maildir()
            && let Some(maildir_config) = &account_config.maildir
        {
            report.backends.push(BackendCheck::from(
                "maildir",
                connect_maildir(maildir_config),
            ));
        }

        #[cfg(feature = "m2dir")]
        if backend.allows_m2dir()
            && let Some(m2dir_config) = &account_config.m2dir
        {
            report
                .backends
                .push(BackendCheck::from("m2dir", connect_m2dir(m2dir_config)));
        }

        #[cfg(feature = "smtp")]
        if backend.allows_smtp()
            && let Some(smtp_config) = &account_config.smtp
        {
            report
                .backends
                .push(BackendCheck::from("smtp", connect_smtp(smtp_config)));
        }

        #[cfg(feature = "sieve")]
        if backend.allows_sieve()
            && let Some(sieve_config) = &account_config.sieve
        {
            report
                .backends
                .push(BackendCheck::from("sieve", connect_sieve(sieve_config)));
        }

        if report.backends.is_empty() {
            bail!("No backend matching `{backend}` is configured for this account");
        }

        printer.out(report)
    }
}

/// Tests every configured backend, failing on the first error.
///
/// The wizard runs it over a freshly built account, so a bad credential
/// or endpoint stops it rather than yielding a configuration that cannot
/// connect.
pub fn test_account(account_config: &AccountConfig) -> Result<()> {
    #[cfg(feature = "imap")]
    if let Some(imap_config) = &account_config.imap {
        connect_imap(imap_config)?;
    }

    #[cfg(feature = "jmap")]
    if let Some(jmap_config) = &account_config.jmap {
        connect_jmap(jmap_config)?;
    }

    #[cfg(feature = "gmail")]
    if let Some(gmail_config) = &account_config.gmail {
        connect_gmail(gmail_config)?;
    }

    #[cfg(feature = "msgraph")]
    if let Some(msgraph_config) = &account_config.msgraph {
        connect_msgraph(msgraph_config)?;
    }

    #[cfg(feature = "maildir")]
    if let Some(maildir_config) = &account_config.maildir {
        connect_maildir(maildir_config)?;
    }

    #[cfg(feature = "m2dir")]
    if let Some(m2dir_config) = &account_config.m2dir {
        connect_m2dir(m2dir_config)?;
    }

    #[cfg(feature = "smtp")]
    if let Some(smtp_config) = &account_config.smtp {
        connect_smtp(smtp_config)?;
    }

    #[cfg(feature = "sieve")]
    if let Some(sieve_config) = &account_config.sieve {
        connect_sieve(sieve_config)?;
    }

    Ok(())
}

/// Opens an authenticated IMAP session and drops it.
#[cfg(feature = "imap")]
pub(crate) fn connect_imap(imap_config: &ImapConfig) -> Result<()> {
    use io_imap::{
        client::{ImapClientStd, default_port},
        session::ImapSessionOpenOptions,
    };
    use io_sasl::mechanism::Sasl;

    use crate::imap::{client::parse_imap_server, id::resolve_auto_id_params};

    let tls = imap_config.tls.clone().into_tls(imap_config.alpn.clone());
    let auto_id = resolve_auto_id_params(&imap_config.id)?;
    let server = parse_imap_server(&imap_config.server)?;
    let sasl: Option<Sasl> = imap_config
        .sasl
        .clone()
        .map(|cfg| {
            let host = server.host_str().unwrap_or_default();
            let port = server.port().unwrap_or(default_port(server.scheme()));
            cfg.try_into_sasl(host, port)
        })
        .transpose()?;
    let opts = ImapSessionOpenOptions {
        starttls: imap_config.starttls,
        auto_id,
        sasl_ir: imap_config.sasl_ir,
    };
    let _ = ImapClientStd::connect(&server, &tls, sasl, opts)?;

    Ok(())
}

/// Reads a server's CAPABILITY over an unauthenticated connection and
/// returns the mechanisms it advertises, most preferred first.
///
/// The wizard offers what comes back rather than the whole list, and the
/// connection is dropped without ever authenticating.
#[cfg(feature = "imap")]
pub(crate) fn probe_imap_mechanisms(server: &str, starttls: bool) -> Result<Vec<SaslMechanism>> {
    use io_imap::{
        client::{ImapClientStd, default_alpn},
        rfc3501::capability::available_auth_mechanisms,
        session::ImapSessionOpenOptions,
    };
    use io_sasl::mechanism::Sasl;

    use crate::{config::TlsConfig, imap::client::parse_imap_server};

    let tls = TlsConfig::default().into_tls(default_alpn());
    let server = parse_imap_server(server)?;
    let opts = ImapSessionOpenOptions {
        starttls,
        ..Default::default()
    };
    let (_client, capabilities) = ImapClientStd::connect(&server, &tls, None::<Sasl>, opts)?;

    Ok(available_auth_mechanisms(&capabilities))
}

/// Opens a JMAP client and fetches the session object.
#[cfg(feature = "jmap")]
fn connect_jmap(jmap_config: &crate::config::JmapConfig) -> Result<()> {
    use io_jmap::client::JmapClientStd;

    use crate::jmap::client::{jmap_http_auth, parse_server_url};

    let tls = jmap_config.tls.clone().into_tls(jmap_config.alpn.clone());
    let http_auth = jmap_http_auth(jmap_config.auth.clone())?;
    let url = parse_server_url(&jmap_config.server)?;
    let mut client = JmapClientStd::connect(&url, &tls, http_auth)?;
    client.session_get(&url)?;

    Ok(())
}

/// Opens a Gmail client and fetches the account profile.
#[cfg(feature = "gmail")]
fn connect_gmail(gmail_config: &crate::config::GmailConfig) -> Result<()> {
    use io_gmail::v1::client::{GmailClientStd, GmailClientStdConnectOptions};
    use secrecy::ExposeSecret;

    use crate::gmail::client::gmail_token;

    let tls = gmail_config.tls.clone().into_tls(gmail_config.alpn.clone());
    let token = gmail_token(gmail_config.auth.clone())?;
    let options = GmailClientStdConnectOptions {
        tls,
        user_id: gmail_config.user_id.clone(),
    };
    let mut client = GmailClientStd::connect(token.expose_secret(), options)?;
    client.profile_get()?;

    Ok(())
}

/// Opens a Microsoft Graph client and fetches the signed-in user.
#[cfg(feature = "msgraph")]
fn connect_msgraph(msgraph_config: &crate::config::MsgraphConfig) -> Result<()> {
    use io_msgraph::v1::client::{MsgraphClientStd, MsgraphClientStdConnectOptions};
    use secrecy::ExposeSecret;

    use crate::msgraph::client::msgraph_token;

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
}

/// Checks that the Maildir root is a directory.
#[cfg(feature = "maildir")]
fn connect_maildir(maildir_config: &crate::config::MaildirConfig) -> Result<()> {
    if !maildir_config.root.is_dir() {
        bail!(
            "Maildir root `{}` does not exist or is not a directory",
            maildir_config.root.display()
        );
    }

    Ok(())
}

/// Checks that the m2dir root is a directory.
#[cfg(feature = "m2dir")]
fn connect_m2dir(m2dir_config: &crate::config::M2dirConfig) -> Result<()> {
    if !m2dir_config.root.is_dir() {
        bail!(
            "m2dir root `{}` does not exist or is not a directory",
            m2dir_config.root.display()
        );
    }

    Ok(())
}

/// Opens an authenticated SMTP session and drops it.
#[cfg(feature = "smtp")]
pub(crate) fn connect_smtp(smtp_config: &crate::config::SmtpConfig) -> Result<()> {
    use std::net::Ipv4Addr;

    use io_sasl::mechanism::Sasl;
    use io_smtp::{
        client::SmtpClientStd, rfc5321::SmtpEhloDomain, session::SmtpSessionOpenOptions,
    };

    let tls = smtp_config.tls.clone().into_tls(smtp_config.alpn.clone());
    let domain: SmtpEhloDomain<'static> = Ipv4Addr::new(127, 0, 0, 1).into();
    let server = crate::smtp::client::parse_smtp_server(&smtp_config.server)?;
    let sasl: Option<Sasl> = smtp_config
        .sasl
        .clone()
        .map(|cfg| {
            let host = server.host_str().unwrap_or_default();
            let port = server
                .port()
                .unwrap_or(SmtpClientStd::default_port(server.scheme()));
            cfg.try_into_sasl(host, port)
        })
        .transpose()?;
    let opts = SmtpSessionOpenOptions {
        starttls: smtp_config.starttls,
    };
    let _client = SmtpClientStd::connect(&server, &tls, domain, sasl, opts)?;

    Ok(())
}

/// Opens an authenticated ManageSieve session and drops it.
#[cfg(feature = "sieve")]
pub(crate) fn connect_sieve(sieve_config: &crate::config::SieveConfig) -> Result<()> {
    let _client = crate::sieve::client::SieveClient::new(sieve_config.clone())?;
    Ok(())
}

/// The `account check` output, one outcome per backend.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct CheckReport {
    /// The account that was checked.
    pub account: String,
    /// One entry per backend the account declares and `--backend` allows.
    pub backends: Vec<BackendCheck>,
}

/// Outcome of checking one backend's connection.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct BackendCheck {
    /// The backend that was checked.
    pub backend: &'static str,
    /// Whether it answered.
    pub ok: bool,
    /// Why it did not, when it did not.
    pub error: Option<String>,
}

impl BackendCheck {
    /// Folds a connection attempt into its outcome.
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

//! # Account check
//!
//! The `account check` command, opening one connection per configured
//! backend so a credential or an endpoint fails here rather than in the
//! middle of a real command.

#[cfg(feature = "mbox")]
use std::fs::File;
use std::{fmt, path::PathBuf};

use anyhow::Result;
#[cfg(any(feature = "maildir", feature = "mbox", feature = "m2dir"))]
use anyhow::bail;
use clap::Parser;
use pimalaya_cli::{error::PartialOutcome, fields::Fields, printer::Printer};
use pimalaya_config::secret::SecretResolver;
use schemars::JsonSchema;
use serde::Serialize;

#[cfg(feature = "imap")]
use crate::config::ImapConfig;
use crate::{backend::Backend, cli::resolve_account, error::ErrorCode};

/// Validate the account configuration.
///
/// Every backend `--backend` allows is opened, which exercises the same
/// handshake and authentication paths a real command takes. The report
/// names each backend and whether it answered.
///
/// The report is printed whole, each backend with its outcome and, when it
/// failed, its code and why. A failed backend exits the command with
/// status 2, the report being the output rather than a failure object.
#[derive(Debug, Parser)]
pub struct AccountCheckCommand;

impl AccountCheckCommand {
    /// Checks every allowed backend of the account and prints the report,
    /// or fails naming every backend that did not answer.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config_paths: &[PathBuf],
        account_name: Option<&str>,
        backend: Backend,
    ) -> Result<()> {
        let (_, name, account_config) = resolve_account(config_paths, account_name)?;

        #[cfg_attr(
            not(any(
                feature = "imap",
                feature = "jmap",
                feature = "gmail",
                feature = "msgraph",
                feature = "maildir",
                feature = "m2dir",
                feature = "mbox",
                feature = "pimdir",
                feature = "smtp",
                feature = "sieve"
            )),
            allow(unused_mut)
        )]
        let mut report = AccountCheckOutput {
            account: name,
            backends: Vec::new(),
        };

        // NOTE: one resolver for the whole account, so a credential
        // command several blocks name is spawned once.
        #[cfg_attr(
            not(any(
                feature = "imap",
                feature = "jmap",
                feature = "gmail",
                feature = "msgraph",
                feature = "smtp",
                feature = "sieve"
            )),
            allow(unused_mut, unused_variables)
        )]
        let mut resolver = SecretResolver::new();

        #[cfg(feature = "imap")]
        if backend.allows_imap()
            && let Some(imap_config) = &account_config.imap
        {
            report.backends.push(BackendCheck::from(
                "imap",
                connect_imap(imap_config, &mut resolver),
            ));
        }

        #[cfg(feature = "jmap")]
        if backend.allows_jmap()
            && let Some(jmap_config) = &account_config.jmap
        {
            report.backends.push(BackendCheck::from(
                "jmap",
                connect_jmap(jmap_config, &mut resolver),
            ));
        }

        #[cfg(feature = "gmail")]
        if backend.allows_gmail()
            && let Some(gmail_config) = &account_config.gmail
        {
            report.backends.push(BackendCheck::from(
                "gmail",
                connect_gmail(gmail_config, &mut resolver),
            ));
        }

        #[cfg(feature = "msgraph")]
        if backend.allows_msgraph()
            && let Some(msgraph_config) = &account_config.msgraph
        {
            report.backends.push(BackendCheck::from(
                "msgraph",
                connect_msgraph(msgraph_config, &mut resolver),
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

        #[cfg(feature = "mbox")]
        if backend.allows_mbox()
            && let Some(mbox_config) = &account_config.mbox
        {
            report
                .backends
                .push(BackendCheck::from("mbox", connect_mbox(mbox_config)));
        }

        #[cfg(feature = "pimdir")]
        if backend.allows_pimdir()
            && let Some(pimdir_config) = &account_config.pimdir
        {
            report
                .backends
                .push(BackendCheck::from("pimdir", connect_pimdir(pimdir_config)));
        }

        #[cfg(feature = "smtp")]
        if backend.allows_smtp()
            && let Some(smtp_config) = &account_config.smtp
        {
            report.backends.push(BackendCheck::from(
                "smtp",
                connect_smtp(smtp_config, &mut resolver),
            ));
        }

        #[cfg(feature = "sieve")]
        if backend.allows_sieve()
            && let Some(sieve_config) = &account_config.sieve
        {
            report.backends.push(BackendCheck::from(
                "sieve",
                connect_sieve(sieve_config, &mut resolver),
            ));
        }

        if report.backends.is_empty() {
            return Err(account_config.no_backend_error(
                &report.account,
                backend,
                Backend::COMPILED,
            ));
        }

        let failed = report.backends.iter().any(|check| !check.ok);
        printer.out(report)?;

        // NOTE: the report names every backend's outcome, so a failed one
        // ends the command with the partial-outcome status rather than a
        // failure object.
        if failed {
            return Err(PartialOutcome::default().into());
        }

        Ok(())
    }
}

/// Opens an authenticated IMAP session and drops it.
#[cfg(feature = "imap")]
fn connect_imap(imap_config: &ImapConfig, resolver: &mut SecretResolver) -> Result<()> {
    use io_imap::{
        client::{ImapClientStd, ImapClientStdConnectOptions, default_port},
        session::ImapSessionOpenOptions,
    };
    use io_sasl::mechanism::Sasl;

    use crate::{
        config::ProxyConfig,
        imap::{client::parse_imap_server, id::resolve_auto_id_params},
    };

    let auto_id = resolve_auto_id_params(&imap_config.id)?;
    let server = parse_imap_server(&imap_config.server)?;
    let sasl: Option<Sasl> = imap_config
        .sasl
        .clone()
        .map(|cfg| {
            let host = server.host_str().unwrap_or_default();
            let port = server.port().unwrap_or(default_port(server.scheme()));
            cfg.try_into_sasl(host, port, resolver)
        })
        .transpose()?;
    let opts = ImapClientStdConnectOptions {
        tls: imap_config.tls.clone().into_tls(imap_config.alpn.clone()),
        proxy: ProxyConfig::resolve(imap_config.proxy.clone(), resolver)?,
        sasl,
        session: ImapSessionOpenOptions {
            starttls: imap_config.starttls,
            auto_id,
            sasl_ir: imap_config.sasl_ir,
        },
    };
    let _ = ImapClientStd::connect(&server, opts)?;

    Ok(())
}

/// Opens a JMAP client and fetches the session object.
#[cfg(feature = "jmap")]
fn connect_jmap(
    jmap_config: &crate::config::JmapConfig,
    resolver: &mut SecretResolver,
) -> Result<()> {
    use io_jmap::client::JmapClientStd;

    use crate::jmap::client::{connect_options, jmap_http_auth, parse_server_url};

    let http_auth = jmap_http_auth(jmap_config.auth.clone(), resolver)?;
    let opts = connect_options(jmap_config, resolver)?;
    let url = parse_server_url(&jmap_config.server)?;
    let mut client = JmapClientStd::connect(&url, http_auth, opts)?;
    client.session_get(&url)?;

    Ok(())
}

/// Opens a Gmail client and fetches the account profile.
#[cfg(feature = "gmail")]
fn connect_gmail(
    gmail_config: &crate::config::GmailConfig,
    resolver: &mut SecretResolver,
) -> Result<()> {
    use io_gmail::v1::client::{GmailClientStd, GmailClientStdConnectOptions};
    use secrecy::ExposeSecret;

    use crate::{config::ProxyConfig, gmail::client::gmail_token};

    let token = gmail_token(gmail_config.auth.clone(), resolver)?;
    let options = GmailClientStdConnectOptions {
        tls: gmail_config.tls.clone().into_tls(gmail_config.alpn.clone()),
        proxy: ProxyConfig::resolve(gmail_config.proxy.clone(), resolver)?,
        user_id: gmail_config.user_id.clone(),
    };
    let mut client = GmailClientStd::connect(token.expose_secret(), options)?;
    client.profile_get()?;

    Ok(())
}

/// Opens a Microsoft Graph client and fetches the signed-in user.
#[cfg(feature = "msgraph")]
fn connect_msgraph(
    msgraph_config: &crate::config::MsgraphConfig,
    resolver: &mut SecretResolver,
) -> Result<()> {
    use io_msgraph::v1::client::{MsgraphClientStd, MsgraphClientStdConnectOptions};
    use secrecy::ExposeSecret;

    use crate::{config::ProxyConfig, msgraph::client::msgraph_token};

    let token = msgraph_token(msgraph_config.auth.clone(), resolver)?;
    let options = MsgraphClientStdConnectOptions {
        tls: msgraph_config
            .tls
            .clone()
            .into_tls(msgraph_config.alpn.clone()),
        proxy: ProxyConfig::resolve(msgraph_config.proxy.clone(), resolver)?,
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

/// Checks that the mbox root is a directory and the spool, when set, a
/// readable file.
#[cfg(feature = "mbox")]
fn connect_mbox(mbox_config: &crate::config::MboxConfig) -> Result<()> {
    if !mbox_config.root.is_dir() {
        bail!(
            "Mbox root `{}` does not exist or is not a directory",
            mbox_config.root.display()
        );
    }

    if let Some(inbox) = &mbox_config.inbox
        && let Err(err) = File::open(inbox)
    {
        bail!("Cannot open mbox spool `{}`: {err}", inbox.display());
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

/// Opens the pimdir store as a reader, lists its mailboxes, then opens a
/// producer and probes it, so a missing, foreign or busy store fails here
/// rather than on the first read or write.
///
/// The probe takes and releases the staging lock without waiting on a
/// collector, and the database's write lock, writing nothing.
#[cfg(feature = "pimdir")]
fn connect_pimdir(pimdir_config: &crate::config::PimdirConfig) -> Result<()> {
    use anyhow::Context;

    let mut client = crate::pimdir::client::PimdirClient::new(pimdir_config.clone())?;
    client.list_mailboxes(false)?;
    client
        .producer()?
        .probe()
        .map_err(crate::error::pimdir_error)
        .with_context(|| format!("Probe pimdir store `{}`", pimdir_config.root.display()))
}

/// Opens an authenticated SMTP session and drops it.
#[cfg(feature = "smtp")]
fn connect_smtp(
    smtp_config: &crate::config::SmtpConfig,
    resolver: &mut SecretResolver,
) -> Result<()> {
    use std::net::Ipv4Addr;

    use io_sasl::mechanism::Sasl;
    use io_smtp::{
        client::{SmtpClientStd, SmtpClientStdConnectOptions},
        rfc5321::SmtpEhloDomain,
        session::SmtpSessionOpenOptions,
    };

    use crate::config::ProxyConfig;

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
            cfg.try_into_sasl(host, port, resolver)
        })
        .transpose()?;
    let opts = SmtpClientStdConnectOptions {
        tls: smtp_config.tls.clone().into_tls(smtp_config.alpn.clone()),
        proxy: ProxyConfig::resolve(smtp_config.proxy.clone(), resolver)?,
        sasl,
        session: SmtpSessionOpenOptions {
            starttls: smtp_config.starttls,
        },
    };
    let _client = SmtpClientStd::connect(&server, domain, opts)?;

    Ok(())
}

/// Opens an authenticated ManageSieve session and drops it.
#[cfg(feature = "sieve")]
fn connect_sieve(
    sieve_config: &crate::config::SieveConfig,
    resolver: &mut SecretResolver,
) -> Result<()> {
    let _client = crate::sieve::client::SieveClient::new(sieve_config.clone(), resolver)?;
    Ok(())
}

/// The `account check` output, one outcome per backend.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountCheckOutput {
    /// The account that was checked.
    pub account: String,
    /// One entry per backend the account declares and `--backend` allows.
    pub backends: Vec<BackendCheck>,
}

/// Outcome of checking one backend's connection.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackendCheck {
    /// The backend that was checked.
    pub backend: &'static str,
    /// Whether it answered.
    pub ok: bool,
    /// The stable code of the failure, `null` when it answered or the
    /// failure carries none.
    pub code: Option<ErrorCode>,
    /// Why it did not answer, `null` when it did.
    pub error: Option<String>,
}

impl BackendCheck {
    /// Folds a connection attempt into its outcome.
    #[cfg(any(
        feature = "imap",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph",
        feature = "maildir",
        feature = "m2dir",
        feature = "mbox",
        feature = "pimdir",
        feature = "smtp",
        feature = "sieve"
    ))]
    fn from(backend: &'static str, result: Result<()>) -> Self {
        match result {
            Ok(()) => Self {
                backend,
                ok: true,
                code: None,
                error: None,
            },
            Err(err) => Self {
                backend,
                ok: false,
                code: pimalaya_cli::error::code_of(&err),
                error: Some(format!("{err:#}")),
            },
        }
    }
}

impl fmt::Display for AccountCheckOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Fields::default();
        fields.push("Account", &self.account);

        for check in &self.backends {
            match &check.error {
                None => fields.push(check.backend, "OK"),
                Some(err) => fields.push(check.backend, format!("FAIL ({err})")),
            }
        }

        write!(f, "{fields}")
    }
}

#[cfg(all(test, feature = "pimdir"))]
mod tests {
    use std::{
        fmt,
        fs::{self, OpenOptions},
    };

    use anyhow::Result;
    use io_pimdir::client::PimdirStore;
    use pimalaya_cli::{
        error::{code_of, partial_outcome_of},
        output::Bytes,
        printer::Printer,
    };
    use serde::Serialize;

    use crate::{
        account::check::{AccountCheckCommand, connect_pimdir},
        backend::Backend,
        config::PimdirConfig,
        error::ErrorCode,
    };

    /// A printer keeping what it is handed, as text.
    #[derive(Default)]
    struct Capture(String);

    impl Printer for Capture {
        fn out<T: fmt::Display + Serialize>(&mut self, data: T) -> Result<()> {
            self.0.push_str(&data.to_string());
            Ok(())
        }

        fn event<T: fmt::Display + Serialize>(&mut self, event: T) -> Result<()> {
            self.0.push_str(&event.to_string());
            Ok(())
        }

        fn out_bytes(&mut self, bytes: Bytes) -> Result<()> {
            self.0.push_str(&String::from_utf8_lossy(&bytes.data));
            Ok(())
        }

        fn is_json(&self) -> bool {
            false
        }
    }

    /// An account reading a pimdir store alone is checked rather than
    /// refused as declaring no backend, and a store that does not answer
    /// prints the report naming the backend and why, then ends with the
    /// partial-outcome status, so a script reads the verdict from the
    /// exit code and each backend from the report.
    #[test]
    fn a_pimdir_account_is_checked() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store");
        let config = dir.path().join("config.toml");
        std::fs::write(
            &config,
            format!(
                "[accounts.test]\ndefault = true\npimdir.root = {:?}\n",
                store.display().to_string(),
            ),
        )
        .unwrap();
        let check = || {
            let mut printer = Capture::default();
            let result = AccountCheckCommand.execute(
                &mut printer,
                std::slice::from_ref(&config),
                None,
                Backend::Auto,
            );
            (result, printer.0)
        };

        let (result, out) = check();
        let err = result.unwrap_err();
        assert_eq!(partial_outcome_of(&err).map(|o| o.status()), Some(2));
        assert!(out.contains("\npimdir:   FAIL (No pimdir store"), "{out}");

        io_pimdir::client::PimdirStore::open(&store)
            .unwrap()
            .ensure_collection("imap/INBOX", "message/rfc822")
            .unwrap();
        let (result, out) = check();
        result.unwrap();
        assert_eq!(out, "Account:  test\npimdir:   OK\n");
    }

    /// A collector holding the staging lock fails the check at once with
    /// `store-busy`, a store the reader opens being one a write may still
    /// not reach, and the check stages nothing into the blob tree.
    #[test]
    fn a_held_staging_lock_fails_the_check_as_busy() {
        let dir = tempfile::tempdir().unwrap();
        PimdirStore::open(dir.path())
            .unwrap()
            .ensure_collection("imap/INBOX", "message/rfc822")
            .unwrap();
        let collector = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.path().join("objects.lock"))
            .unwrap();
        collector.try_lock().unwrap();

        let config = PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        };
        let err = connect_pimdir(&config).unwrap_err();
        assert_eq!(code_of(&err), Some(ErrorCode::StoreBusy), "{err:#}");

        drop(collector);
        connect_pimdir(&config).unwrap();

        let objects = dir.path().join("objects");
        let left: Vec<_> = match fs::read_dir(&objects) {
            Ok(entries) => entries.map(|entry| entry.unwrap().path()).collect(),
            Err(_) => Vec::new(),
        };
        assert!(left.is_empty(), "{left:?}");
    }
}

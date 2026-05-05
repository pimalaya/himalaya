//! Builder for the unified [`io_email::client::EmailClient`] used by
//! cross-protocol shared subcommands (`mailboxes`, `envelopes`,
//! `flags`, `messages`).
//!
//! The legacy per-backend dispatch — three nearly-identical
//! `if backend.allows_X() { … resume loop … }` blocks per command —
//! is replaced by a single call to [`build`] that returns a fully
//! authenticated [`EmailContext`]. The shared command then calls one
//! [`EmailClient`] method and renders the result.
//!
//! Construction is still backend-asymmetric (IMAP needs TLS + SASL,
//! JMAP needs an HTTP credential, Maildir just needs a root path),
//! and that asymmetry is collapsed here. We delegate to the existing
//! transitional [`ImapSession`] / [`JmapSession`] helpers for the
//! handshake/auth flow, then bridge the resulting `(stream, context)`
//! pairs into [`io_imap::client::ImapClient`] / [`io_jmap::client::JmapClient`]
//! via their `from_parts` constructors.
//!
//! [`ImapSession`]: crate::imap::session::ImapSession
//! [`JmapSession`]: crate::jmap::session::JmapSession

use std::path::PathBuf;

use anyhow::{bail, Result};
use comfy_table::ContentArrangement;
use io_email::client::EmailClient;

use crate::{
    account::Account,
    cli::BackendArg,
    config::{AccountConfig, Config},
};

/// Bundle handed to shared commands: a fully-built [`EmailClient`]
/// plus the account-level rendering settings the per-backend
/// dispatchers used to extract independently.
pub struct EmailContext {
    pub client: EmailClient,
    #[allow(dead_code)]
    pub downloads_dir: PathBuf,
    pub table_preset: String,
    pub table_arrangement: ContentArrangement,
}

/// Builds an [`EmailContext`] from `(config, account_config, backend)`.
///
/// Tries each backend in `imap → jmap → maildir` order, picking the
/// first one whose config block is present and whose [`BackendArg`]
/// filter allows it. Bails when nothing matches. SMTP is omitted on
/// purpose: none of the shared read-side operations have an SMTP
/// implementation.
pub fn build(
    config: Config,
    mut account_config: AccountConfig,
    backend: BackendArg,
) -> Result<EmailContext> {
    #[cfg(feature = "imap")]
    if backend.allows_imap() {
        if let Some(imap_config) = account_config.imap.take() {
            use crate::imap::session::ImapSession;
            use io_imap::client::ImapClient;

            let account = Account::new(config, account_config, imap_config)?;
            let session = ImapSession::new(
                account.backend.url.clone(),
                account.backend.tls.clone().try_into()?,
                account.backend.starttls,
                account.backend.sasl.clone().try_into()?,
            )?;
            let client = ImapClient::from_parts(session.stream, session.context);
            return Ok(EmailContext {
                client: client.into(),
                downloads_dir: account.downloads_dir,
                table_preset: account.table_preset,
                table_arrangement: account.table_arrangement,
            });
        }
    }

    #[cfg(feature = "jmap")]
    if backend.allows_jmap() {
        if let Some(jmap_config) = account_config.jmap.take() {
            use crate::jmap::session::JmapSession;
            use io_jmap::client::JmapClient;

            let account = Account::new(config, account_config, jmap_config)?;
            let session = JmapSession::new(
                account.backend.server.clone(),
                account.backend.tls.clone().try_into()?,
                account.backend.auth.clone().try_into()?,
            )?;
            let client = JmapClient::from_parts(session.stream, session.http_auth, session.session);
            return Ok(EmailContext {
                client: client.into(),
                downloads_dir: account.downloads_dir,
                table_preset: account.table_preset,
                table_arrangement: account.table_arrangement,
            });
        }
    }

    #[cfg(feature = "maildir")]
    if backend.allows_maildir() {
        if let Some(maildir_config) = account_config.maildir.take() {
            use io_maildir::client::MaildirClient;

            let account = Account::new(config, account_config, maildir_config)?;
            let client = MaildirClient::new(account.backend.root.clone());
            return Ok(EmailContext {
                client: client.into(),
                downloads_dir: account.downloads_dir,
                table_preset: account.table_preset,
                table_arrangement: account.table_arrangement,
            });
        }
    }

    bail!("no backend matching `{backend}` is configured for this account")
}

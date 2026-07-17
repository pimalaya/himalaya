//! Himalaya wrapper around [`io_gmail::v1::client::GmailClientStd`] plus
//! the credential helper shared with the cross-protocol client
//! ([`crate::shared::client`]) and the account checker
//! ([`crate::account::check`]).
//!
//! The shared API ([`crate::shared::client::EmailClient`]) covers the
//! least-common-denominator operations over Gmail; the protocol-
//! specific `himalaya gmail` command uses [`GmailClient`] directly to
//! expose the full Gmail REST surface (labels, threads, drafts,
//! history, raw message/attachment access).
//!
//! [`GmailClientStd::connect`]: io_gmail::v1::client::GmailClientStd::connect

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use io_gmail::v1::client::{GmailClientStd as Inner, GmailClientStdConnectOptions};
use secrecy::{ExposeSecret, SecretString};

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, GmailAuthConfig, GmailConfig},
};

/// Live Gmail client handed down to every `gmail` subcommand.
pub struct GmailClient {
    inner: Inner,
    /// Lazily-fetched `(id, name)` pairs for every label, used by
    /// [`Self::resolve_mailbox_id`] to map the shared layer's
    /// human-facing label names onto opaque Gmail label ids. Cached for
    /// the client's lifetime so a `copy`/`move` resolves both endpoints
    /// with a single `labels.list`.
    label_index: Option<Vec<(String, String)>>,
}

impl GmailClient {
    /// Opens a TLS connection to the Gmail REST API
    /// (`https://gmail.googleapis.com`) with the configured bearer
    /// credential and user id.
    pub fn new(config: GmailConfig) -> Result<Self> {
        let tls = config.tls.into_tls(config.alpn);
        let token = gmail_token(config.auth)?;
        let options = GmailClientStdConnectOptions {
            tls,
            user_id: config.user_id,
        };
        let inner = Inner::connect(token.expose_secret(), options)?;
        Ok(Self {
            inner,
            label_index: None,
        })
    }

    /// Maps a human label name to its opaque Gmail label id, for the
    /// shared backend which otherwise addresses mailboxes (labels) by
    /// their id.
    ///
    /// A value already matching a known id passes through untouched (id
    /// passthrough); an exact display-name match returns the mapped id
    /// (first match wins); an unknown value is handed back as-is so the
    /// API surfaces the error. The label index is fetched once
    /// (`labels.list`) and cached.
    ///
    /// Lives here so the backend operation methods stay pure id
    /// consumers: name resolution never happens inside them.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        if self.label_index.is_none() {
            let labels = self.labels_list()?.response.labels;
            let index = labels
                .into_iter()
                .map(|label| (label.id, label.name))
                .collect();
            self.label_index = Some(index);
        }

        let index = self.label_index.as_deref().unwrap_or_default();

        if index.iter().any(|(id, _)| id == mailbox) {
            return Ok(mailbox.to_string());
        }

        if let Some((id, _)) = index.iter().find(|(_, name)| name == mailbox) {
            return Ok(id.clone());
        }

        Ok(mailbox.to_string())
    }
}

impl Deref for GmailClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for GmailClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Opens the Gmail client for an already-resolved account: takes the
/// `[gmail]` block out of `account_config` and builds the merged
/// [`Account`]. Bails when the account has no `[gmail]` block.
pub fn build_gmail_client(
    config: Config,
    name: String,
    mut account_config: AccountConfig,
) -> Result<(Account, GmailClient)> {
    let gmail_config = account_config
        .gmail
        .take()
        .ok_or_else(|| anyhow!("Gmail config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(account_config));
    let client = GmailClient::new(gmail_config)?;
    Ok((account, client))
}

/// Resolves a [`GmailAuthConfig`] into the bare OAuth 2.0 bearer token;
/// the Gmail client adds the `Bearer ` prefix itself.
pub fn gmail_token(config: GmailAuthConfig) -> Result<SecretString> {
    Ok(config.token.get()?)
}

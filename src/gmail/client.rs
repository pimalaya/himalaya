//! # Gmail client
//!
//! The wrapper around io-gmail's blocking client every Gmail-specific
//! subcommand receives, plus the credential helper the shared client and
//! the account checker take.
//!
//! The shared API covers the least-common-denominator operations, where
//! the `gmail` command reaches for the full REST surface through this
//! wrapper.

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use io_gmail::v1::client::{GmailClientStd as Inner, GmailClientStdConnectOptions};
use secrecy::{ExposeSecret, SecretString};

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, GmailAuthConfig, GmailConfig},
};

/// A live Gmail client and the label index it caches.
pub struct GmailClient {
    inner: Inner,
    /// The `(id, name)` pairs [`Self::resolve_mailbox_id`] maps names
    /// through, fetched once and cached for the client's lifetime.
    label_index: Option<Vec<(String, String)>>,
}

impl GmailClient {
    /// Opens a TLS connection to the Gmail REST API with the configured
    /// bearer credential and user id.
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

    /// Maps a human label name onto its opaque Gmail label id.
    ///
    /// A known id passes through, a name match returns its id, and an
    /// unknown value goes back as it is so the API surfaces the error. It
    /// lives here so every backend method stays a pure id consumer.
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

/// Opens the Gmail client of an already-resolved account, returning it
/// beside the merged [`Account`].
///
/// Bails when the account declares no `[gmail]` block.
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

/// Resolves the configuration into the bare OAuth 2.0 token, the client
/// adding the `Bearer ` prefix itself.
pub fn gmail_token(config: GmailAuthConfig) -> Result<SecretString> {
    Ok(config.token.get()?)
}

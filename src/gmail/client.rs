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
        Ok(Self { inner })
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

//! Himalaya wrapper around [`io_msgraph::v1::client::MsgraphClientStd`]
//! plus the credential helper shared with the cross-protocol client
//! ([`crate::shared::client`]) and the account checker
//! ([`crate::account::check`]).
//!
//! The shared API ([`io_email::client::EmailClientStd`]) covers the
//! least-common-denominator operations over Microsoft Graph; the
//! protocol-specific `himalaya msgraph` command uses [`MsgraphClient`]
//! directly to expose the Graph mail surface (mail folders, messages,
//! attachments, raw MIME access).
//!
//! [`MsgraphClientStd::connect`]: io_msgraph::v1::client::MsgraphClientStd::connect

use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use io_msgraph::v1::client::{MsgraphClientStd as Inner, MsgraphClientStdConnectOptions};
use pimalaya_config::toml::TomlConfig;
use secrecy::{ExposeSecret, SecretString};

use crate::{
    account::context::Account, cli::load_or_wizard, config::MsgraphAuthConfig,
    config::MsgraphConfig,
};

/// Live Microsoft Graph client handed down to every `msgraph` subcommand.
pub struct MsgraphClient {
    inner: Inner,
}

impl MsgraphClient {
    /// Opens a TLS connection to the Microsoft Graph API
    /// (`https://graph.microsoft.com`) with the configured bearer
    /// credential and user id.
    pub fn new(config: MsgraphConfig) -> Result<Self> {
        let tls = config.tls.into_tls(config.alpn);
        let token = msgraph_token(config.auth)?;
        let options = MsgraphClientStdConnectOptions {
            tls,
            user_id: config.user_id,
        };
        let inner = Inner::connect(token.expose_secret(), options)?;
        Ok(Self { inner })
    }
}

impl Deref for MsgraphClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MsgraphClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Loads the configuration, picks the active account, builds the merged
/// [`Account`] then opens the Microsoft Graph client. Bails when the
/// account has no `[msgraph]` block.
pub fn build_msgraph_client(
    config_paths: &[PathBuf],
    account_name: Option<&str>,
) -> Result<(Account, MsgraphClient)> {
    let mut config = load_or_wizard(config_paths)?;
    let (name, mut ac) = config
        .take_account(account_name)?
        .ok_or_else(|| anyhow!("Cannot find account"))?;
    let msgraph_config = ac
        .msgraph
        .take()
        .ok_or_else(|| anyhow!("Microsoft Graph config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(ac));
    let client = MsgraphClient::new(msgraph_config)?;
    Ok((account, client))
}

/// Resolves a [`MsgraphAuthConfig`] into the bare OAuth 2.0 bearer token;
/// the Microsoft Graph client adds the `Bearer ` prefix itself.
pub fn msgraph_token(config: MsgraphAuthConfig) -> Result<SecretString> {
    Ok(config.token.get()?)
}

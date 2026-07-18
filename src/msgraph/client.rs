//! Himalaya wrapper around [`io_msgraph::v1::client::MsgraphClientStd`]
//! plus the credential helper shared with the cross-protocol client
//! ([`crate::shared::client`]) and the account checker
//! ([`crate::account::check`]).
//!
//! The shared API ([`crate::shared::client::EmailClient`]) covers the
//! least-common-denominator operations over Microsoft Graph; the
//! protocol-specific `himalaya msgraph` command uses [`MsgraphClient`]
//! directly to expose the Graph mail surface (mail folders, messages,
//! attachments, raw MIME access).
//!
//! [`MsgraphClientStd::connect`]: io_msgraph::v1::client::MsgraphClientStd::connect

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use io_msgraph::v1::{
    client::{MsgraphClientStd as Inner, MsgraphClientStdConnectOptions},
    rest::users::mail_folders::list::MsgraphMailFoldersListParams,
};
use secrecy::{ExposeSecret, SecretString};

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, MsgraphAuthConfig, MsgraphConfig},
};

/// Live Microsoft Graph client handed down to every `msgraph` subcommand.
pub struct MsgraphClient {
    inner: Inner,
    /// Lazily-fetched `(id, name)` pairs for every mail folder, used by
    /// [`Self::resolve_mailbox_id`] to map the shared layer's
    /// human-facing folder names onto opaque Graph folder ids. Cached for
    /// the client's lifetime so a `copy`/`move` resolves both endpoints
    /// with a single `mailFolders` listing.
    folder_index: Option<Vec<(String, String)>>,
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
        Ok(Self {
            inner,
            folder_index: None,
        })
    }

    /// Maps a human folder name to its opaque Graph folder id, for the
    /// shared backend which otherwise addresses mailboxes (folders) by
    /// their id.
    ///
    /// A value already matching a known id passes through untouched (id
    /// passthrough); an exact display-name match returns the mapped id
    /// (first match wins); an unknown value is handed back as-is, so a
    /// Graph well-known name (`inbox`, `archive`, `sentitems`, …) still
    /// reaches the API and any other value surfaces the API error. The
    /// folder index is fetched once (`mailFolders`) and cached.
    ///
    /// Lives here so the backend operation methods stay pure id
    /// consumers: name resolution never happens inside them.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        if self.folder_index.is_none() {
            let params = MsgraphMailFoldersListParams {
                top: Some(100),
                ..Default::default()
            };
            let folders = self.mail_folders_list(&params)?.response.value;
            let index = folders
                .into_iter()
                .map(|folder| (folder.id, folder.display_name))
                .collect();
            self.folder_index = Some(index);
        }

        let index = self.folder_index.as_deref().unwrap_or_default();

        if index.iter().any(|(id, _)| id == mailbox) {
            return Ok(mailbox.to_string());
        }

        if let Some((id, _)) = index.iter().find(|(_, name)| name == mailbox) {
            return Ok(id.clone());
        }

        Ok(mailbox.to_string())
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

/// Opens the Microsoft Graph client for an already-resolved account:
/// takes the `[msgraph]` block out of `account_config` and builds the
/// merged [`Account`]. Bails when the account has no `[msgraph]` block.
pub fn build_msgraph_client(
    config: Config,
    name: String,
    mut account_config: AccountConfig,
) -> Result<(Account, MsgraphClient)> {
    let msgraph_config = account_config
        .msgraph
        .take()
        .ok_or_else(|| anyhow!("Microsoft Graph config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(account_config));
    let client = MsgraphClient::new(msgraph_config)?;
    Ok((account, client))
}

/// Resolves a [`MsgraphAuthConfig`] into the bare OAuth 2.0 bearer token;
/// the Microsoft Graph client adds the `Bearer ` prefix itself.
pub fn msgraph_token(config: MsgraphAuthConfig) -> Result<SecretString> {
    Ok(config.token.get()?)
}

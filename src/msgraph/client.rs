//! # Microsoft Graph client
//!
//! The wrapper around io-msgraph's blocking client every Graph-specific
//! subcommand receives, plus the credential helper the shared client and
//! the account checker take.
//!
//! The shared API covers the least-common-denominator operations, where
//! the `msgraph` command reaches for the Graph mail surface through this
//! wrapper.

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use io_msgraph::v1::client::{MsgraphClientStd as Inner, MsgraphClientStdConnectOptions};
use pimalaya_config::secret::SecretResolver;
use secrecy::{ExposeSecret, SecretString};

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, MsgraphAuthConfig, MsgraphConfig, ProxyConfig},
    email::mailbox::{MailboxIndex, MailboxRole},
};

/// A live Microsoft Graph client and the folder index it caches.
pub struct MsgraphClient {
    inner: Inner,
    /// The folders [`Self::resolve_mailbox_id`] maps names and roles
    /// through, fetched once and cached for the client's lifetime.
    folder_index: Option<MailboxIndex>,
}

impl MsgraphClient {
    /// Opens a TLS connection to the Microsoft Graph API
    /// (`https://graph.microsoft.com`) with the configured bearer
    /// credential and user id.
    pub fn new(config: MsgraphConfig) -> Result<Self> {
        let mut resolver = SecretResolver::new();
        let token = msgraph_token(config.auth, &mut resolver)?;
        let options = MsgraphClientStdConnectOptions {
            tls: config.tls.into_tls(config.alpn),
            proxy: ProxyConfig::resolve(config.proxy, &mut resolver)?,
            user_id: config.user_id,
        };
        let inner = Inner::connect(token.expose_secret(), options)?;
        Ok(Self {
            inner,
            folder_index: None,
        })
    }

    /// Maps a folder id, name or role onto its opaque Graph folder id.
    ///
    /// An unknown value goes back as it is, so a Graph well-known name
    /// still reaches the API. It lives here so every backend method stays
    /// a pure id consumer.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        self.folder_index()?.resolve(mailbox)
    }

    /// The id of the folder carrying `role`, `None` when none does.
    pub fn role_mailbox_id(&mut self, role: &MailboxRole) -> Result<Option<String>> {
        self.folder_index()?.with_role(role)
    }

    fn folder_index(&mut self) -> Result<&MailboxIndex> {
        if self.folder_index.is_none() {
            let folders = self.list_mailboxes(false)?;
            self.folder_index = Some(MailboxIndex(folders));
        }

        Ok(self.folder_index.get_or_insert_default())
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
///
/// The token goes through `resolver`, so an account naming one command
/// here and in another block spawns it once.
pub fn msgraph_token(
    config: MsgraphAuthConfig,
    resolver: &mut SecretResolver,
) -> Result<SecretString> {
    Ok(resolver.resolve(config.token)?)
}

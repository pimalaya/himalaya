//! # Microsoft Graph client
//!
//! The wrapper around io-msgraph's blocking client every Graph-specific
//! subcommand receives, plus the credential helper the shared client and
//! the account checker take.
//!
//! The unified API covers the operations every mail backend names, where
//! the `msgraph` command reaches for the Graph mail surface through this
//! wrapper.

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow, bail};
use io_msgraph::v1::{
    client::{MsgraphClientStd as Inner, MsgraphClientStdConnectOptions, MsgraphClientStdError},
    send::MsgraphSendError,
};
use pimalaya_config::secret::SecretResolver;
use secrecy::{ExposeSecret, SecretString};

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, MsgraphAuthConfig, MsgraphConfig, ProxyConfig},
    email::mailbox::{MailboxIndex, MailboxRole},
    msgraph::backend::well_known_role,
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

    /// Maps a folder id, path or role onto its opaque Graph folder id.
    ///
    /// A well-known name (`msgfolderroot`, `outbox`) or a folder id the
    /// listing does not hold (a hidden folder) is asked of Graph; any
    /// other value, or one Graph knows no folder by, is refused. It lives
    /// here so every backend method stays a pure id consumer.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        let index = self.folder_index()?;
        let id = index.resolve(mailbox)?;

        if index.0.iter().any(|m| m.id == id) {
            return Ok(id);
        }

        // NOTE: a well-known name maps onto the folder carrying its role,
        // so it compares equal to that folder's id.
        if let Some(role) = well_known_role(&id)
            && let Some(id) = index.with_role(&role)?
        {
            return Ok(id);
        }

        if !may_name_folder(&id) {
            bail!("Mailbox `{mailbox}` not found");
        }

        match self.mail_folder_get(&id) {
            Ok(folder) => Ok(folder.response.id),
            Err(MsgraphClientStdError::Send(err)) if names_no_folder(&err) => {
                bail!("Mailbox `{mailbox}` not found")
            }
            Err(err) => Err(err.into()),
        }
    }

    /// The id of the folder carrying `role`, `None` when none does.
    pub fn role_mailbox_id(&mut self, role: &MailboxRole) -> Result<Option<String>> {
        self.folder_index()?.with_role(role)
    }

    /// Drops the cached folders, so the next lookup sees a folder created,
    /// renamed or deleted since.
    pub fn forget_mailboxes(&mut self) {
        self.folder_index = None;
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
    let account = Account {
        name: Some(name.to_string()),
        ..Account::from(config).merge(Account::from(account_config))
    };
    let client = MsgraphClient::new(msgraph_config)?;
    Ok((account, client))
}

/// The well-known folder names Graph v1.0 accepts in place of an id.
const WELL_KNOWN_NAMES: [&str; 17] = [
    "archive",
    "clutter",
    "conflicts",
    "conversationhistory",
    "deleteditems",
    "drafts",
    "inbox",
    "junkemail",
    "localfailures",
    "msgfolderroot",
    "outbox",
    "recoverableitemsdeletions",
    "scheduled",
    "searchfolders",
    "sentitems",
    "serverfailures",
    "syncissues",
];

/// Whether `value` may name a Graph folder: a well-known name, or shaped
/// like a folder id (long, base64url).
fn may_name_folder(value: &str) -> bool {
    let id_like = value.len() >= 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_=".contains(&byte));

    id_like || WELL_KNOWN_NAMES.contains(&value.to_ascii_lowercase().as_str())
}

/// Whether Graph's answer to a folder lookup means no such folder: a
/// malformed id (400), an unknown one (404), or a value no path segment
/// can carry.
fn names_no_folder(err: &MsgraphSendError) -> bool {
    matches!(err.status(), Some(400 | 404)) || matches!(err, MsgraphSendError::InvalidRequest(_))
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

#[cfg(test)]
mod tests {
    use crate::msgraph::client::may_name_folder;

    #[test]
    fn only_a_well_known_name_or_an_id_shape_is_asked_of_graph() {
        assert!(may_name_folder("msgfolderroot"));
        assert!(may_name_folder("Outbox"));
        assert!(may_name_folder(
            "AAMkADczZWY5NjZhLTY4NzMtNGM4OS1hZGZiLWZmNzdiNGU3YzUxZgAuAAAAAAAwyvki6t3BQokRIq4u7tKhAQC_mCAVB5-SQqiAnuFDrYg6AAAAAAELAAA="
        ));
        assert!(!may_name_folder("No-Such-Folder"));
        assert!(!may_name_folder("Himalaya-Test-MS-A/Sub"));
        assert!(!may_name_folder("a/b?c#d"));
    }
}

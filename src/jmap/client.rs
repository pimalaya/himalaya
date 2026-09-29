//! # JMAP client
//!
//! The wrapper around io-jmap's blocking client every JMAP-specific
//! subcommand receives.
//!
//! The dispatch layer opens the session up front and hands the ready
//! wrapper down, the merged [`Account`] riding along as a sibling
//! argument.

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use base64::{Engine, prelude::BASE64_STANDARD};
use io_jmap::client::{JmapClientStd as Inner, JmapClientStdConnectOptions};
use pimalaya_config::secret::SecretResolver;
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, JmapAuthConfig, JmapConfig, ProxyConfig, parse_server},
    email::mailbox::{MailboxIndex, MailboxRole},
};

/// A live JMAP session and the mailbox index it caches.
pub struct JmapClient {
    inner: Inner,
    /// The `[jmap]` block, kept so a command can open an auxiliary
    /// session of its own against an upload or download authority the
    /// API one does not cover.
    pub config: JmapConfig,
    /// The mailboxes [`Self::resolve_mailbox_id`] maps names and roles
    /// through, fetched once and cached for the client's lifetime.
    mailbox_index: Option<MailboxIndex>,
}

impl JmapClient {
    /// Establishes the session, discovering the endpoint through
    /// `/.well-known/jmap` when the configuration names an authority.
    pub fn new(config: JmapConfig) -> Result<Self> {
        let mut resolver = SecretResolver::new();
        let http_auth = jmap_http_auth(config.auth.clone(), &mut resolver)?;
        let opts = connect_options(&config, &mut resolver)?;
        let url = parse_server_url(&config.server)?;

        let mut inner = Inner::connect(&url, http_auth, opts)?;
        inner.session_get(&url)?;

        Ok(Self {
            inner,
            config,
            mailbox_index: None,
        })
    }

    /// Maps a mailbox id, name or role onto its opaque JMAP id.
    ///
    /// It lives here so every backend method stays a pure id consumer.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        self.mailbox_index()?.resolve(mailbox)
    }

    /// The id of the mailbox carrying `role`, `None` when none does.
    pub fn role_mailbox_id(&mut self, role: &MailboxRole) -> Result<Option<String>> {
        self.mailbox_index()?.with_role(role)
    }

    fn mailbox_index(&mut self) -> Result<&MailboxIndex> {
        if self.mailbox_index.is_none() {
            let mailboxes = self.list_mailboxes(false)?;
            self.mailbox_index = Some(MailboxIndex(mailboxes));
        }

        Ok(self.mailbox_index.get_or_insert_default())
    }

    /// Downloads a blob, whose URL may live on another authority.
    ///
    /// A matching host reuses the live session, a foreign one opens a
    /// fresh authenticated connection. Reusing the API socket would send
    /// the request to the API server, which answers with a redirect no
    /// download follows.
    pub fn download_blob(&mut self, download_url: &Url) -> Result<Vec<u8>> {
        let api_url = {
            let session = self
                .session()
                .ok_or_else(|| anyhow!("JMAP session is missing"))?;
            session.api_url.clone()
        };

        if same_authority(&api_url, download_url) {
            return Ok(self.blob_download(download_url)?);
        }

        let mut resolver = SecretResolver::new();
        let http_auth = jmap_http_auth(self.config.auth.clone(), &mut resolver)?;
        let opts = connect_options(&self.config, &mut resolver)?;
        let mut download_client = Inner::connect(download_url, http_auth, opts)?;

        Ok(download_client.blob_download(download_url)?)
    }
}

/// Whether two URLs share host and effective port, i.e. a live
/// connection to one can carry a request for the other.
fn same_authority(a: &Url, b: &Url) -> bool {
    a.host() == b.host() && a.port_or_known_default() == b.port_or_known_default()
}

impl Deref for JmapClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for JmapClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Opens the JMAP session of an already-resolved account, returning it
/// beside the merged [`Account`].
///
/// Bails when the account declares no `[jmap]` block.
pub fn build_jmap_client(
    config: Config,
    name: String,
    mut account_config: AccountConfig,
) -> Result<(Account, JmapClient)> {
    let jmap_config = account_config
        .jmap
        .take()
        .ok_or_else(|| anyhow!("JMAP config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(account_config));
    let client = JmapClient::new(jmap_config)?;
    Ok((account, client))
}

/// Parses the JMAP `server` field into a [`Url`]. Accepts a full
/// `http`/`https://host[:port][/path]` URL, a bare `host:port`, or a
/// bare `host`; the last two default to `https://` (secure). Any other
/// scheme is rejected.
pub fn parse_server_url(server: &str) -> Result<Url> {
    parse_server(server, "https", &["http", "https"])
}

/// Builds the connect options of a `[jmap]` block, its proxy password
/// going through `resolver`.
pub fn connect_options(
    config: &JmapConfig,
    resolver: &mut SecretResolver,
) -> Result<JmapClientStdConnectOptions> {
    Ok(JmapClientStdConnectOptions {
        tls: config.tls.clone().into_tls(config.alpn.clone()),
        proxy: ProxyConfig::resolve(config.proxy.clone(), resolver)?,
    })
}

/// Converts a [`JmapAuthConfig`] into the pre-formatted HTTP
/// `Authorization` header value [`JmapClientStd::connect`] expects.
///
/// The credential goes through `resolver`, so an account naming one
/// command here and in another block spawns it once.
///
/// [`JmapClientStd::connect`]: io_jmap::client::JmapClientStd::connect
pub fn jmap_http_auth(
    config: JmapAuthConfig,
    resolver: &mut SecretResolver,
) -> Result<SecretString> {
    match config {
        JmapAuthConfig::Header(token) => Ok(resolver.resolve(token)?),
        JmapAuthConfig::Bearer { token } => {
            let token = resolver.resolve(token)?;
            Ok(format!("Bearer {}", token.expose_secret()).into())
        }
        JmapAuthConfig::Basic { username, password } => {
            let creds = format!(
                "{}:{}",
                username,
                resolver.resolve(password)?.expose_secret()
            );
            let encoded = BASE64_STANDARD.encode(creds.into_bytes());
            Ok(format!("Basic {encoded}").into())
        }
    }
}

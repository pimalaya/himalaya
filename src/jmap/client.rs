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
use io_jmap::{client::JmapClientStd as Inner, rfc8621::mailbox::get::JmapMailboxGetOptions};
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, JmapAuthConfig, JmapConfig, parse_server},
};

/// A live JMAP session and the mailbox index it caches.
pub struct JmapClient {
    inner: Inner,
    /// The `[jmap]` block, kept so a command can open an auxiliary
    /// session of its own against an upload or download authority the
    /// API one does not cover.
    pub config: JmapConfig,
    /// The `(id, name)` pairs [`Self::resolve_mailbox_id`] maps names
    /// through, fetched once and cached for the client's lifetime.
    mailbox_index: Option<Vec<(String, String)>>,
}

impl JmapClient {
    /// Establishes the session, discovering the endpoint through
    /// `/.well-known/jmap` when the configuration names an authority.
    pub fn new(config: JmapConfig) -> Result<Self> {
        let tls = config.tls.clone().into_tls(config.alpn.clone());

        let http_auth = jmap_http_auth(config.auth.clone())?;
        let url = parse_server_url(&config.server)?;

        let mut inner = Inner::connect(&url, &tls, http_auth)?;
        inner.session_get(&url)?;

        Ok(Self {
            inner,
            config,
            mailbox_index: None,
        })
    }

    /// Maps a human mailbox name onto its opaque JMAP id.
    ///
    /// A known id passes through, a name match returns its id, and an
    /// unknown value goes back as it is so the server surfaces the error.
    /// It lives here so every backend method stays a pure id consumer.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        if self.mailbox_index.is_none() {
            let output = self.mailbox_get(JmapMailboxGetOptions {
                ids: None,
                properties: None,
            })?;
            let index = output
                .mailboxes
                .into_iter()
                .filter_map(|mailbox| Some((mailbox.id?, mailbox.name.unwrap_or_default())))
                .collect();
            self.mailbox_index = Some(index);
        }

        let index = self.mailbox_index.as_deref().unwrap_or_default();

        if index.iter().any(|(id, _)| id == mailbox) {
            return Ok(mailbox.to_string());
        }

        if let Some((id, _)) = index.iter().find(|(_, name)| name == mailbox) {
            return Ok(id.clone());
        }

        Ok(mailbox.to_string())
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

        let tls = self.config.tls.clone().into_tls(self.config.alpn.clone());
        let http_auth = jmap_http_auth(self.config.auth.clone())?;
        let mut download_client = Inner::connect(download_url, &tls, http_auth)?;

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

/// Converts a [`JmapAuthConfig`] into the pre-formatted HTTP
/// `Authorization` header value [`JmapClientStd::connect`] expects.
///
/// [`JmapClientStd::connect`]: io_jmap::client::JmapClientStd::connect
pub fn jmap_http_auth(config: JmapAuthConfig) -> Result<SecretString> {
    match config {
        JmapAuthConfig::Header(token) => Ok(token.get()?),
        JmapAuthConfig::Bearer { token } => {
            let token = token.get()?;
            Ok(format!("Bearer {}", token.expose_secret()).into())
        }
        JmapAuthConfig::Basic { username, password } => {
            let creds = format!("{}:{}", username, password.get()?.expose_secret());
            let encoded = BASE64_STANDARD.encode(creds.into_bytes());
            Ok(format!("Basic {encoded}").into())
        }
    }
}

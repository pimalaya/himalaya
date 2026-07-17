//! Himalaya wrapper around [`io_jmap::client::JmapClientStd`] that
//! bundles the merged [`Account`] alongside the live JMAP client.
//!
//! Built up front by the dispatch layer (`crate::cli`) via
//! [`build_jmap_client`] and handed down to every JMAP-specific
//! subcommand.

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

/// Live JMAP session paired with the merged account configuration.
pub struct JmapClient {
    inner: Inner,
    /// The original JMAP config block, kept around so commands like
    /// `email import` / `email export` can spin up their own
    /// auxiliary sessions (e.g. against the upload/download URL when
    /// it lives on a different authority than the API URL).
    pub config: JmapConfig,
    /// Lazily-fetched `(id, name)` pairs for every mailbox, used by
    /// [`Self::resolve_mailbox_id`] to map the shared layer's
    /// human-facing mailbox names onto opaque JMAP ids. Cached for the
    /// client's lifetime so a `copy`/`move` resolves both endpoints in
    /// a single `Mailbox/get`.
    mailbox_index: Option<Vec<(String, String)>>,
}

impl JmapClient {
    /// Establishes the JMAP session (TLS, `/.well-known/jmap`
    /// discovery).
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

    /// Maps a human mailbox name to its opaque JMAP id, for the shared
    /// backend which otherwise addresses mailboxes by their id.
    ///
    /// A value that already matches a known id is returned verbatim (id
    /// passthrough, mirroring IMAP where the name *is* the id); an exact
    /// display-name match returns the mapped id (first match wins on the
    /// rare duplicate-name case); an unknown value is handed back as-is
    /// so the server surfaces the error. The mailbox index is fetched
    /// once (`Mailbox/get`) and cached.
    ///
    /// This lives here, on the Himalaya client, precisely so the backend
    /// operation methods (`list_envelopes`, `add_message`, …) stay pure
    /// id consumers: name resolution never happens inside them.
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

    /// Downloads a blob whose URL may live on a different authority than
    /// the JMAP API endpoint. Fastmail, for one, serves downloads from
    /// `*.fastmailusercontent.com` while the API is on `api.fastmail.com`.
    ///
    /// When the download host matches the API host the live session
    /// connection is reused; otherwise a fresh authenticated connection
    /// is opened to the download host. The API socket is *never* reused
    /// for a foreign host — doing so sends the download request to the
    /// API server, which (Fastmail) answers with a `302` to its docs
    /// page and fails the non-redirectable download.
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

/// Opens the JMAP session for an already-resolved account: takes the
/// `[jmap]` block out of `account_config`, builds the merged [`Account`]
/// and connects. Bails when the account has no `[jmap]` block. Returns
/// the live client paired with the merged account so subcommands receive
/// both as sibling arguments.
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

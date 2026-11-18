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
use io_jmap::client::{JmapClientStd as Inner, JmapClientStdConnectOptions, JmapClientStdError};
use pimalaya_config::secret::SecretResolver;
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::{
    account::context::Account,
    config::{
        AccountConfig, Config,
        jmap::{JmapAuthConfig, JmapConfig},
        proxy::ProxyConfig,
    },
    email::mailbox::{MailboxIndex, MailboxRole},
    error::mailbox_not_found,
    server::parse_server,
    shared::base64,
};

/// A live JMAP session and the mailbox index it caches.
pub struct JmapClient {
    inner: Inner,
    /// The `[jmap]` block the session was opened from.
    pub config: JmapConfig,
    /// The `Authorization` value, resolved once with the session, so an
    /// auxiliary connection spawns no credential command again.
    http_auth: SecretString,
    /// The connect options, the proxy password resolved once.
    options: JmapClientStdConnectOptions,
    /// The connection to an upload or download authority the API one
    /// does not cover, kept for the next blob on that authority.
    auxiliary: Option<(Url, Inner)>,
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

        let mut inner = Inner::connect(&url, http_auth.clone(), opts.clone())?;
        inner.session_get(&url)?;

        Ok(Self {
            inner,
            config,
            http_auth,
            options: opts,
            auxiliary: None,
            mailbox_index: None,
        })
    }

    /// Maps a mailbox id, name or role onto its opaque JMAP id.
    ///
    /// It lives here so every backend method stays a pure id consumer.
    /// JMAP naming no mailbox natively, a value matching none is refused
    /// here, the server only answering `invalidArguments` for it.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        let index = self.mailbox_index()?;
        let id = index.resolve(mailbox)?;

        if !index.0.iter().any(|m| m.id == id) {
            return Err(mailbox_not_found(format!("Mailbox `{mailbox}` not found")));
        }

        Ok(id)
    }

    /// The id of the mailbox carrying `role`, `None` when none does.
    pub fn role_mailbox_id(&mut self, role: &MailboxRole) -> Result<Option<String>> {
        self.mailbox_index()?.with_role(role)
    }

    /// Drops the cached mailboxes, so the next lookup sees a mailbox
    /// created, renamed or deleted since.
    pub fn forget_mailboxes(&mut self) {
        self.mailbox_index = None;
    }

    fn mailbox_index(&mut self) -> Result<&MailboxIndex> {
        if self.mailbox_index.is_none() {
            let mailboxes = self.list_mailboxes(false)?.value;
            self.mailbox_index = Some(MailboxIndex(mailboxes));
        }

        Ok(self.mailbox_index.get_or_insert_default())
    }

    /// Downloads a blob, whose URL may live on another authority.
    ///
    /// A matching host reuses the live session, a foreign one the
    /// auxiliary connection to it ([`Self::auxiliary`]). Reusing the API
    /// socket would send the request to the API server, which answers with
    /// a redirect no download follows.
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

        self.auxiliary(download_url, |client| client.blob_download(download_url))
    }

    /// Runs `request` over the connection to the authority of `url`, one
    /// the API session does not cover, opened with the credential the
    /// session resolved and kept for the next request there. A request
    /// failing over a kept connection, which the server may have closed
    /// since, is sent once more over a fresh one.
    pub fn auxiliary<T>(
        &mut self,
        url: &Url,
        mut request: impl FnMut(&mut Inner) -> Result<T, JmapClientStdError>,
    ) -> Result<T> {
        if let Some((authority, mut client)) = self.auxiliary.take()
            && same_authority(&authority, url)
            && let Ok(answer) = request(&mut client)
        {
            self.auxiliary = Some((authority, client));
            return Ok(answer);
        }

        let mut client = Inner::connect(url, self.http_auth.clone(), self.options.clone())?;
        let answer = request(&mut client)?;
        self.auxiliary = Some((url.clone(), client));
        Ok(answer)
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
    let account = Account {
        name: Some(name.to_string()),
        ..Account::from(config).merge(Account::from(account_config))
    };
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
        tls: config.tls.clone().into_tls(config.alpn.clone())?,
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
            let encoded = base64::encode(creds.as_bytes());
            Ok(format!("Basic {encoded}").into())
        }
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_config::secret::{Secret, SecretResolver};
    use secrecy::{ExposeSecret, SecretString};

    use crate::{
        config::jmap::JmapAuthConfig,
        jmap::client::{jmap_http_auth, parse_server_url},
    };

    #[test]
    fn basic_auth_is_standard_padded_base64() {
        let auth = JmapAuthConfig::Basic {
            username: String::from("user"),
            password: Secret::Raw(SecretString::from("pa\u{e9}ss")),
        };
        let header = jmap_http_auth(auth, &mut SecretResolver::new()).unwrap();
        assert_eq!(header.expose_secret(), "Basic dXNlcjpwYcOpc3M=");
    }

    #[test]
    fn a_non_ascii_host_is_refused() {
        assert!(parse_server_url("https://b\u{fc}cher.example/jmap").is_err());
        assert!(parse_server_url("b\u{fc}cher.example").is_err());

        let url = parse_server_url("https://xn--bcher-kva.example/jmap").unwrap();
        assert_eq!(url.host_str(), Some("xn--bcher-kva.example"));
        let url = parse_server_url("xn--bcher-kva.example").unwrap();
        assert_eq!(url.host_str(), Some("xn--bcher-kva.example"));
    }

    /// Blob downloads on an authority other than the API's run the
    /// credential command once, with the session, and share one
    /// connection.
    #[test]
    fn blob_downloads_reuse_the_credential_and_the_connection() {
        use std::{
            io::{BufRead, BufReader, Write},
            net::TcpListener,
            sync::{
                Arc,
                atomic::{AtomicUsize, Ordering},
            },
            thread,
        };

        use crate::{config::jmap::JmapConfig, jmap::client::JmapClient};

        fn answer(listener: TcpListener, body: String, accepted: Arc<AtomicUsize>) {
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                accepted.fetch_add(1, Ordering::SeqCst);
                let body = body.clone();
                thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut out = stream;
                    let mut line = String::new();
                    while reader.read_line(&mut line).unwrap_or(0) > 0 {
                        if line == "\r\n" {
                            let len = body.len();
                            let head = "HTTP/1.1 200 OK\r\nContent-Type: application/json";
                            write!(out, "{head}\r\nContent-Length: {len}\r\n\r\n{body}").unwrap();
                        }
                        line.clear();
                    }
                });
            }
        }

        let api = TcpListener::bind("127.0.0.1:0").unwrap();
        let blobs = TcpListener::bind("127.0.0.1:0").unwrap();
        let api_url = format!("http://{}", api.local_addr().unwrap());
        let blob_url = format!("http://{}", blobs.local_addr().unwrap());
        let session = serde_json::json!({
            "username": "user",
            "accounts": {},
            "primaryAccounts": { "urn:ietf:params:jmap:mail": "A" },
            "capabilities": { "urn:ietf:params:jmap:core": {}, "urn:ietf:params:jmap:mail": {} },
            "apiUrl": format!("{api_url}/api"),
            "downloadUrl": format!("{blob_url}/download/{{accountId}}/{{blobId}}"),
            "uploadUrl": format!("{api_url}/upload/{{accountId}}"),
            "eventSourceUrl": format!("{api_url}/events"),
            "state": "s",
        });
        let downloads = Arc::new(AtomicUsize::new(0));
        let accepted = downloads.clone();
        thread::spawn(move || answer(api, session.to_string(), Arc::default()));
        thread::spawn(move || answer(blobs, String::from("blob"), accepted));

        let dir = tempfile::tempdir().unwrap();
        let runs = dir.path().join("runs");
        let command = format!("echo run >> '{}'; echo token", runs.display());
        let config = format!("server = \"{api_url}/session\"\nauth.header.command = {command:?}\n");
        let config: JmapConfig = toml::from_str(&config).unwrap();

        let mut client = JmapClient::new(config).unwrap();
        for blob in ["B1", "B2"] {
            let url = url::Url::parse(&format!("{blob_url}/download/A/{blob}")).unwrap();
            assert_eq!(client.download_blob(&url).unwrap(), b"blob");
        }

        let runs = std::fs::read_to_string(runs).unwrap();
        assert_eq!(runs.lines().count(), 1, "{runs}");
        assert_eq!(downloads.load(Ordering::SeqCst), 1);
    }
}

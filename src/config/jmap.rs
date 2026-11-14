//! # JMAP configuration
//!
//! The `jmap` block of an account.

use pimalaya_config::{secret::Secret, toml::shell_expanded_string};
use serde::Deserialize;

use crate::config::{proxy::ProxyConfig, tls::TlsConfig};

/// JMAP configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct JmapConfig {
    /// The JMAP server address, a bare authority or a full URL.
    ///
    /// A bare authority is discovered through `GET /.well-known/jmap`,
    /// a full URL reaching the session endpoint directly. The schemes are
    /// `http` and `https`, a bare authority taking `https`. The host is
    /// ASCII, an internationalised one written in Punycode (`xn--…`).
    pub server: String,
    /// TLS provider and custom certificate used by the connection.
    #[serde(default)]
    pub tls: TlsConfig,
    /// Proxy the connection goes through, falling back to the account one
    /// and then to the `all_proxy`/`https_proxy` environment variables.
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// `["http/1.1"]`, JMAP riding on HTTP/1.1.
    ///
    /// An empty list skips ALPN negotiation. Only rustls reads it,
    /// `native-tls` ignoring ALPN.
    #[serde(default = "default_jmap_alpn")]
    pub alpn: Vec<String>,
    /// Authentication configuration.
    pub auth: JmapAuthConfig,
    /// Identity id `message send` submits under, required for JMAP send
    /// alone and discoverable with `himalaya jmap identity get`.
    pub identity_id: Option<String>,
    /// Mailbox id `message send` stages a message in before submitting it.
    ///
    /// Required for JMAP send alone, and discoverable with `himalaya jmap
    /// mailbox query --role drafts`.
    pub drafts_mailbox_id: Option<String>,
    /// Mailbox id a sent message is moved to once submitted, defaulting
    /// to the `sent`-role mailbox.
    ///
    /// Discoverable with `himalaya jmap mailbox query --role sent`.
    pub sent_mailbox_id: Option<String>,
}

/// JMAP authentication configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum JmapAuthConfig {
    /// A full `Authorization` header value, sent verbatim.
    Header(Secret),
    /// An OAuth 2.0 bearer token.
    Bearer {
        /// The access token.
        token: Secret,
    },
    /// HTTP Basic authentication.
    Basic {
        /// The account to authenticate as.
        #[serde(deserialize_with = "shell_expanded_string")]
        username: String,
        /// Its password.
        password: Secret,
    },
}

fn default_jmap_alpn() -> Vec<String> {
    vec![String::from("http/1.1")]
}

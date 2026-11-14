//! # Gmail configuration
//!
//! The `gmail` block of an account.

use pimalaya_config::secret::Secret;
use serde::Deserialize;

use crate::config::{proxy::ProxyConfig, tls::TlsConfig};

/// Gmail REST API configuration.
///
/// Gmail has no per-account server address, the client always talking to
/// gmail.googleapis.com, so only the mailbox owner, TLS and the OAuth 2.0
/// credential are configured.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct GmailConfig {
    /// The mailbox owner, defaulting to `me`, the authenticated user.
    #[serde(default = "default_gmail_user_id")]
    pub user_id: String,
    /// TLS provider and custom certificate used by the connection.
    #[serde(default)]
    pub tls: TlsConfig,
    /// Proxy the connection goes through, falling back to the account one
    /// and then to the `all_proxy`/`https_proxy` environment variables.
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// `["http/1.1"]`, the REST API riding on HTTP/1.1.
    ///
    /// An empty list skips ALPN negotiation. Only rustls reads it,
    /// `native-tls` ignoring ALPN.
    #[serde(default = "default_gmail_alpn")]
    pub alpn: Vec<String>,
    /// Authentication configuration.
    pub auth: GmailAuthConfig,
}

/// Gmail authentication configuration.
///
/// Gmail accepts OAuth 2.0 bearer tokens alone, so this is a short-lived
/// access token an external helper such as ortie mints. Refreshing it is
/// the caller's business.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct GmailAuthConfig {
    /// The access token, sent as `Bearer <token>`.
    pub token: Secret,
}

fn default_gmail_user_id() -> String {
    String::from("me")
}

fn default_gmail_alpn() -> Vec<String> {
    vec![String::from("http/1.1")]
}

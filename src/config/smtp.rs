//! # SMTP configuration
//!
//! The `smtp` block of an account.

use serde::Deserialize;

use crate::config::{proxy::ProxyConfig, sasl::SaslConfig, tls::TlsConfig};

/// SMTP configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SmtpConfig {
    /// SMTP server address, a bare authority or a full URL.
    ///
    /// A bare authority takes `smtps://`, implicit TLS. A full URL is used
    /// verbatim, `smtp://` being cleartext with an optional STARTTLS
    /// upgrade. Mirrors [`JmapConfig::server`].
    ///
    /// [`JmapConfig::server`]: crate::config::jmap::JmapConfig::server
    pub server: String,
    /// TLS provider and custom certificate used by the connection.
    #[serde(default)]
    pub tls: TlsConfig,
    /// Proxy the connection goes through, falling back to the account one
    /// and then to the `all_proxy`/`https_proxy` environment variables.
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
    /// Whether to upgrade the connection with `STARTTLS` after the
    /// greeting, valid only for an `smtp://` server.
    #[serde(default)]
    pub starttls: bool,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// the RFC 7595 registered `["smtp"]`.
    ///
    /// An empty list skips ALPN negotiation. Only rustls reads it,
    /// `native-tls` ignoring ALPN.
    #[serde(default = "default_smtp_alpn")]
    pub alpn: Vec<String>,
    /// SASL credentials, see [`ImapConfig::sasl`].
    ///
    /// [`ImapConfig::sasl`]: crate::config::imap::ImapConfig::sasl
    pub sasl: Option<SaslConfig>,
}

fn default_smtp_alpn() -> Vec<String> {
    vec![String::from("smtp")]
}

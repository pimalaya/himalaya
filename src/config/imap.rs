//! # IMAP configuration
//!
//! The `imap` block of an account.

use std::collections::HashMap;

use serde::Deserialize;

use crate::config::{proxy::ProxyConfig, sasl::SaslConfig, tls::TlsConfig};

/// IMAP configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapConfig {
    /// IMAP server address, a bare authority or a full URL.
    ///
    /// A bare authority takes `imaps://`, implicit TLS. A full URL is used
    /// verbatim, `imap://` being cleartext with an optional STARTTLS
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
    /// greeting, valid only for an `imap://` server.
    #[serde(default)]
    pub starttls: bool,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// the RFC 7595 registered `["imap"]`.
    ///
    /// An empty list skips ALPN negotiation. Only rustls reads it,
    /// `native-tls` ignoring ALPN.
    #[serde(default = "default_imap_alpn")]
    pub alpn: Vec<String>,
    /// SASL credentials, omitted to skip authentication entirely.
    ///
    /// No `AUTHENTICATE` command is then sent at all. Advertising the
    /// ANONYMOUS mechanism is `sasl.anonymous = {}` instead.
    pub sasl: Option<SaslConfig>,
    /// RFC 4959 SASL-IR quirk, unset following the advertised capability.
    ///
    /// `false` waits for the server's continuation request rather than
    /// inlining the credentials, which Coremail (126.com, 163.com) needs:
    /// it advertises SASL-IR falsely.
    #[serde(default)]
    pub sasl_ir: Option<bool>,
    /// RFC 2971 `ID` extension quirks.
    ///
    /// Some providers, notably mail.qq.com and fastmail, want an `ID`
    /// exchange straight after authentication: `id.auto = true` opts in.
    #[serde(default)]
    pub id: ImapIdConfig,
    /// RFC 5256 `SORT` extension options.
    #[serde(default)]
    pub sort: ImapSortConfig,
}

/// Per-account `imap.sort.*` options.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapSortConfig {
    /// Forces the client-side sort fallback on or off.
    ///
    /// On, the client sorts with SEARCH and FETCH; off, it always issues a
    /// server `SORT`. Unset, the fallback runs when the server lacks the
    /// SORT capability, and on iCloud, whose SORT ignores its keys.
    pub fallback: Option<bool>,
}

/// Per-account `imap.id.*` quirks.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapIdConfig {
    /// Whether the auth coroutine chains an `ID` round-trip after the
    /// tagged auth response, default `false` skipping it.
    #[serde(default)]
    pub auto: bool,
    /// Parameters sent with the auto-`ID` command.
    ///
    /// Unset sends himalaya's canned value for every well-known key
    /// (`name`, `version`, `vendor`, `support-url`), and an empty map
    /// sends `ID NIL`. `true` substitutes the canned value for a
    /// well-known key and `NIL` otherwise, `false` always sends `NIL`,
    /// and an absent key is not transmitted.
    #[serde(default)]
    pub fields: Option<HashMap<String, bool>>,
}

fn default_imap_alpn() -> Vec<String> {
    vec![String::from("imap")]
}

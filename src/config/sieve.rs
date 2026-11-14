//! # ManageSieve configuration
//!
//! The `sieve` block of an account.

use serde::Deserialize;

use crate::config::{proxy::ProxyConfig, sasl::SaslConfig, tls::TlsConfig};

/// ManageSieve configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SieveConfig {
    /// ManageSieve server address, a bare authority or a full URL.
    ///
    /// RFC 5804 registers one port and reaches TLS on it through STARTTLS,
    /// so a bare authority takes `sieve://` unlike the IMAP and SMTP ones.
    /// `sieves://` is for the deployments the specification does not
    /// define, listening for a handshake straight away.
    pub server: String,
    /// TLS provider and custom certificate used by the connection.
    #[serde(default)]
    pub tls: TlsConfig,
    /// Proxy the connection goes through, falling back to the account one
    /// and then to the `all_proxy`/`https_proxy` environment variables.
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
    /// Whether to upgrade the connection with `STARTTLS` after the
    /// greeting, unset following the scheme.
    ///
    /// On for `sieve://`, the only upgrade path RFC 5804 defines, off for
    /// `sieves://` and `unix://`. Setting it on `sieves://` is an error,
    /// TLS being already up.
    #[serde(default)]
    pub starttls: Option<bool>,
    /// ALPN identifiers offered during the TLS handshake, defaulting to
    /// none since ManageSieve registers none.
    #[serde(default = "default_sieve_alpn")]
    pub alpn: Vec<String>,
    /// Whether a mechanism disclosing a reusable credential may run over a
    /// cleartext connection.
    ///
    /// `plain`, `login`, `oauthbearer` and `xoauth2` hand a passive
    /// observer something it can replay, so they are refused unless the
    /// connection is encrypted. Set this for a trusted local link.
    #[serde(default)]
    pub allow_cleartext_auth: bool,
    /// SASL credentials, see [`ImapConfig::sasl`].
    ///
    /// [`ImapConfig::sasl`]: crate::config::imap::ImapConfig::sasl
    pub sasl: Option<SaslConfig>,
}

fn default_sieve_alpn() -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use crate::config::sieve::SieveConfig;

    #[test]
    fn sieve_config_defaults_to_no_alpn_and_refuses_cleartext_auth() {
        let config: SieveConfig = toml::from_str(
            r#"
                server = "sieve.example.com:4190"
            "#,
        )
        .unwrap();

        assert_eq!(config.server, "sieve.example.com:4190");
        assert_eq!(config.starttls, None);
        assert!(config.alpn.is_empty());
        assert!(!config.allow_cleartext_auth);
        assert!(config.sasl.is_none());
    }
}

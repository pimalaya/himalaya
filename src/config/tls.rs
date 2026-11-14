//! # TLS configuration
//!
//! The `tls` block of a network backend.

use std::path::PathBuf;

use pimalaya_config::toml::{opt_shell_expanded_path, shell_expanded_path};
use serde::Deserialize;

/// SSL/TLS configuration.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TlsConfig {
    /// The TLS implementation, defaulting to rustls.
    pub provider: Option<TlsProviderConfig>,
    /// Rustls-only options.
    #[serde(default)]
    pub rustls: RustlsConfig,
    /// A custom certificate to trust, in PEM format.
    #[serde(default, deserialize_with = "opt_shell_expanded_path")]
    pub cert: Option<PathBuf>,
    /// A client certificate, presented when the server asks for one.
    pub client: Option<TlsClientConfig>,
}

/// Client certificate configuration, both in PEM format.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TlsClientConfig {
    /// The certificate, optionally followed by its intermediates.
    #[serde(deserialize_with = "shell_expanded_path")]
    pub cert: PathBuf,
    /// The private key, PKCS#8 under native-tls.
    #[serde(deserialize_with = "shell_expanded_path")]
    pub key: PathBuf,
}

/// SSL/TLS provider configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum TlsProviderConfig {
    /// The pure-Rust rustls stack.
    Rustls,
    /// The platform's own TLS stack.
    NativeTls,
}

/// Rustls configuration.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RustlsConfig {
    /// The cryptographic provider rustls runs on.
    pub crypto: Option<RustlsCryptoConfig>,
}

/// Rustls crypto provider configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum RustlsCryptoConfig {
    /// aws-lc-rs.
    Aws,
    /// ring.
    Ring,
}

#[cfg(test)]
mod tests {
    use std::env::var;

    use crate::config::tls::*;

    #[test]
    fn optional_paths_expand_the_leading_tilde_and_stay_absent() {
        let home = PathBuf::from(var("HOME").expect("HOME must be set"));

        let tls: TlsConfig = toml::from_str(r#"cert = "~/ca.pem""#).unwrap();
        assert_eq!(tls.cert, Some(home.join("ca.pem")));

        let tls: TlsConfig = toml::from_str("").unwrap();
        assert_eq!(tls.cert, None);
        assert!(tls.client.is_none());

        let tls: TlsConfig =
            toml::from_str(r#"client = { cert = "~/me.pem", key = "~/me.key.pem" }"#).unwrap();
        let client = tls.client.unwrap();
        assert_eq!(client.cert, home.join("me.pem"));
        assert_eq!(client.key, home.join("me.key.pem"));

        // NOTE: a certificate without its key cannot be presented
        assert!(toml::from_str::<TlsConfig>(r#"client.cert = "~/me.pem""#).is_err());
    }
}

//! # Proxy configuration
//!
//! The `proxy` block of an account or of one of its network backends.

use pimalaya_config::secret::Secret;
use serde::Deserialize;

/// Proxy configuration.
///
/// `url` is a `socks5://`, `socks5h://` or `http://` proxy URL. Its user
/// info authenticates too, but `username` and `password` keep the secret
/// out of the URL.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ProxyConfig {
    /// The proxy URL.
    pub url: String,
    /// The proxy username, required by `password`.
    pub username: Option<String>,
    /// The proxy password.
    pub password: Option<Secret>,
}

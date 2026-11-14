//! # TLS
//!
//! Turns the `[*.tls]` configuration into the runtime [`Tls`] handle the
//! network backends connect with.

use pimalaya_stream::tls::{Rustls, RustlsCrypto, Tls, TlsClient, TlsProvider, TlsServer};

use crate::config::tls::{RustlsCryptoConfig, TlsConfig, TlsProviderConfig};

impl TlsConfig {
    /// Builds the runtime [`Tls`] handle the connect helpers expect,
    /// folding in the protocol-level `alpn` list.
    ///
    /// The TOML schema never exposes `tls.rustls.alpn` directly, the
    /// per-protocol `*.alpn` field standing for it. An empty list skips
    /// ALPN.
    pub fn into_tls(self, alpn: Vec<String>) -> Tls {
        Tls {
            provider: self.provider.map(|p| match p {
                TlsProviderConfig::Rustls => TlsProvider::Rustls,
                TlsProviderConfig::NativeTls => TlsProvider::NativeTls,
            }),
            rustls: Rustls {
                crypto: self.rustls.crypto.map(|c| match c {
                    RustlsCryptoConfig::Aws => RustlsCrypto::Aws,
                    RustlsCryptoConfig::Ring => RustlsCrypto::Ring,
                }),
                alpn,
            },
            server: self.cert.map(|cert| TlsServer { cert }),
            client: self.client.map(|c| TlsClient {
                cert: c.cert,
                key: c.key,
            }),
        }
    }
}

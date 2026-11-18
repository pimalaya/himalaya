//! # TLS
//!
//! Turns the `[*.tls]` configuration into the runtime [`Tls`] handle the
//! network backends connect with.

use std::{fs::File, path::Path};

use anyhow::{Context, Result};
use pimalaya_stream::tls::{Rustls, RustlsCrypto, Tls, TlsClient, TlsProvider, TlsServer};

use crate::config::tls::{RustlsCryptoConfig, TlsConfig, TlsProviderConfig};

impl TlsConfig {
    /// Builds the runtime [`Tls`] handle the connect helpers expect,
    /// folding in the protocol-level `alpn` list.
    ///
    /// The TOML schema never exposes `tls.rustls.alpn` directly, the
    /// per-protocol `*.alpn` field standing for it. An empty list skips
    /// ALPN.
    ///
    /// A certificate or key file that cannot be read is refused, its path
    /// named, before any connection.
    pub fn into_tls(self, alpn: Vec<String>) -> Result<Tls> {
        if let Some(cert) = &self.cert {
            readable(cert, "TLS certificate")?;
        }
        if let Some(client) = &self.client {
            readable(&client.cert, "TLS client certificate")?;
            readable(&client.key, "TLS client key")?;
        }

        Ok(Tls {
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
        })
    }
}

/// Refuses the `what` file at `path` when it cannot be read, naming it.
fn readable(path: &Path, what: &str) -> Result<()> {
    File::open(path)
        .map(|_| ())
        .with_context(|| format!("Cannot read {what} `{}`", path.display()))
}

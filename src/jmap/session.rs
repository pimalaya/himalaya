//! Transitional JMAP session helper ported from `pimalaya-toolbox`.
//!
//! Will be replaced by `io_jmap::client::JmapClient` once the
//! protocol-specific subcommands switch over.

use std::{
    io::{Read, Write},
    net::TcpStream,
};

use anyhow::{bail, Result};
use base64::{prelude::BASE64_STANDARD, Engine};
use io_jmap::rfc8620::{
    session::JmapSession as IoJmapSession,
    session_get::{JmapSessionGet, JmapSessionGetResult},
};
use log::info;
use pimalaya_stream::{
    std::stream::Stream,
    tls::{upgrade_tls, Tls},
};
use secrecy::{ExposeSecret, SecretString};
use url::Url;

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Authentication for a JMAP session.
// https://www.iana.org/assignments/http-authschemes/http-authschemes.xhtml#authschemes
#[derive(Clone, Debug)]
pub enum JmapAuth {
    Header(SecretString),
    /// Bearer token (OAuth 2.0).
    Bearer(SecretString),
    /// HTTP Basic authentication.
    Basic {
        username: String,
        password: SecretString,
    },
}

impl From<JmapAuth> for SecretString {
    fn from(auth: JmapAuth) -> SecretString {
        match auth {
            JmapAuth::Header(auth) => auth,
            JmapAuth::Bearer(token) => {
                let token = token.expose_secret();
                format!("Bearer {token}").into()
            }
            JmapAuth::Basic { username, password } => {
                let creds = format!("{}:{}", username, password.expose_secret());
                let creds = BASE64_STANDARD.encode(creds.into_bytes());
                format!("Basic {creds}").into()
            }
        }
    }
}

/// A live JMAP session over a TLS connection.
#[derive(Debug)]
pub struct JmapSession {
    pub session: IoJmapSession,
    pub stream: Stream,
    pub http_auth: SecretString,
}

fn use_tls(scheme: &str) -> bool {
    scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("jmaps")
}

fn default_port(scheme: &str) -> u16 {
    if use_tls(scheme) {
        443
    } else {
        80
    }
}

fn connect(url: &Url, tls: &Tls) -> Result<Stream> {
    let host = url.host_str().unwrap_or("localhost");
    let port = url.port().unwrap_or_else(|| default_port(url.scheme()));
    let tcp = TcpStream::connect((host, port))?;

    if use_tls(url.scheme()) {
        upgrade_tls(host, tcp, tls, &[b"http/1.1"])
    } else {
        Ok(Stream::Tcp(tcp))
    }
}

impl JmapSession {
    /// Establishes a JMAP session.
    pub fn new(server: String, tls: Tls, auth: JmapAuth) -> Result<Self> {
        let url = match Url::parse(&server) {
            Ok(url) => url,
            Err(url::ParseError::RelativeUrlWithoutBase) => {
                Url::parse(&format!("https://{server}"))?
            }
            Err(e) => return Err(e.into()),
        };

        info!("connecting to JMAP server {url}");

        match url.scheme() {
            s if s.eq_ignore_ascii_case("https") || s.eq_ignore_ascii_case("jmaps") => {}
            s if s.eq_ignore_ascii_case("http") || s.eq_ignore_ascii_case("jmap") => {}
            scheme => bail!("unsupported JMAP scheme `{scheme}`, expected http/https/jmap/jmaps"),
        }

        let mut stream = connect(&url, &tls)?;

        let http_auth: SecretString = auth.into();
        let mut coroutine = JmapSessionGet::new(&http_auth, &url);
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let session = loop {
            match coroutine.resume(arg.take()) {
                JmapSessionGetResult::Ok { session, .. } => break session,
                JmapSessionGetResult::WantsRead => {
                    let n = stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapSessionGetResult::WantsWrite(bytes) => {
                    stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapSessionGetResult::WantsRedirect { url: new_url, .. } => {
                    stream = connect(&new_url, &tls)?;
                    coroutine = JmapSessionGet::new(&http_auth, &new_url);
                    arg = None;
                }
                JmapSessionGetResult::Err(err) => return Err(err.into()),
            }
        };

        Ok(Self {
            session,
            stream,
            http_auth,
        })
    }
}

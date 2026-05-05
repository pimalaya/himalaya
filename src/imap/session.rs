//! Transitional IMAP session helper ported from `pimalaya-toolbox`.
//!
//! Will be replaced by `io_imap::client::ImapClient` once the
//! protocol-specific subcommands switch over.

#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::{
    io::{Read, Write},
    net::TcpStream,
};

use anyhow::{bail, Result};
use io_imap::{
    context::ImapContext,
    rfc3501::{
        capability::{ImapCapabilityGet, ImapCapabilityGetResult},
        greeting_with_capability::{
            ImapGreetingWithCapabilityGet, ImapGreetingWithCapabilityGetResult,
        },
        login::{ImapSessionLogin, ImapSessionLoginParams, ImapSessionLoginResult},
        starttls::{ImapStartTls, ImapStartTlsResult},
    },
    sasl::authenticate_plain::{
        ImapSessionAuthenticatePlain, ImapSessionAuthenticatePlainParams,
        ImapSessionAuthenticatePlainResult,
    },
    types::response::Capability,
};
use log::info;
use pimalaya_stream::{
    sasl::{Sasl, SaslMechanism},
    std::stream::Stream,
    tls::{upgrade_tls, Tls},
};
#[cfg(windows)]
use uds_windows::UnixStream;
use url::Url;

const READ_BUFFER_SIZE: usize = 16 * 1024;

#[derive(Debug)]
pub struct ImapSession {
    pub context: ImapContext,
    pub stream: Stream,
}

fn drive_greeting_with_capability<S: Read + Write>(
    stream: &mut S,
    context: ImapContext,
) -> Result<ImapContext> {
    let mut buf = [0u8; READ_BUFFER_SIZE];
    let mut coroutine = ImapGreetingWithCapabilityGet::new(context);
    let mut arg: Option<&[u8]> = None;

    loop {
        match coroutine.resume(arg.take()) {
            ImapGreetingWithCapabilityGetResult::Ok(context) => return Ok(context),
            ImapGreetingWithCapabilityGetResult::WantsRead => {
                let n = stream.read(&mut buf)?;
                arg = Some(&buf[..n]);
            }
            ImapGreetingWithCapabilityGetResult::WantsWrite(bytes) => {
                stream.write_all(&bytes)?;
                arg = None;
            }
            ImapGreetingWithCapabilityGetResult::Err { err, .. } => bail!(err),
        }
    }
}

fn drive_capability<S: Read + Write>(stream: &mut S, context: ImapContext) -> Result<ImapContext> {
    let mut buf = [0u8; READ_BUFFER_SIZE];
    let mut coroutine = ImapCapabilityGet::new(context);
    let mut arg: Option<&[u8]> = None;

    loop {
        match coroutine.resume(arg.take()) {
            ImapCapabilityGetResult::Ok(context) => return Ok(context),
            ImapCapabilityGetResult::WantsRead => {
                let n = stream.read(&mut buf)?;
                arg = Some(&buf[..n]);
            }
            ImapCapabilityGetResult::WantsWrite(bytes) => {
                stream.write_all(&bytes)?;
                arg = None;
            }
            ImapCapabilityGetResult::Err { err, .. } => bail!(err),
        }
    }
}

fn drive_starttls<S: Read + Write>(stream: &mut S, context: ImapContext) -> Result<ImapContext> {
    let mut buf = [0u8; READ_BUFFER_SIZE];
    let mut coroutine = ImapStartTls::new(context);
    let mut arg: Option<&[u8]> = None;

    loop {
        match coroutine.resume(arg.take()) {
            ImapStartTlsResult::WantsStartTls { context, .. } => return Ok(context),
            ImapStartTlsResult::WantsRead => {
                let n = stream.read(&mut buf)?;
                arg = Some(&buf[..n]);
            }
            ImapStartTlsResult::WantsWrite(bytes) => {
                stream.write_all(&bytes)?;
                arg = None;
            }
            ImapStartTlsResult::Err { err, .. } => bail!(err),
        }
    }
}

impl ImapSession {
    pub fn new(url: Url, tls: Tls, starttls: bool, mut sasl: Sasl) -> Result<Self> {
        info!("connecting to IMAP server using {url}");

        let context = ImapContext::new();
        let host = url.host_str().unwrap_or("127.0.0.1");

        let (mut context, mut stream) = match url.scheme() {
            scheme if scheme.eq_ignore_ascii_case("imap") => {
                let port = url.port().unwrap_or(143);
                let mut tcp = TcpStream::connect((host, port))?;
                let context = drive_greeting_with_capability(&mut tcp, context)?;
                (context, Stream::Tcp(tcp))
            }
            scheme if scheme.eq_ignore_ascii_case("imaps") => {
                let port = url.port().unwrap_or(993);
                let mut tcp = TcpStream::connect((host, port))?;

                let context = if starttls {
                    drive_starttls(&mut tcp, context)?
                } else {
                    context
                };

                let mut stream = upgrade_tls(host, tcp, &tls, &[b"imap"])?;

                let context = if starttls {
                    drive_capability(&mut stream, context)?
                } else {
                    drive_greeting_with_capability(&mut stream, context)?
                };

                (context, stream)
            }
            scheme if scheme.eq_ignore_ascii_case("unix") => {
                let sock_path = url.path();
                let mut unix = UnixStream::connect(sock_path)?;
                let context = drive_greeting_with_capability(&mut unix, context)?;
                (context, Stream::Unix(unix))
            }
            scheme => {
                bail!("Unknown scheme {scheme}, expected imap, imaps or unix");
            }
        };

        if !context.authenticated {
            let ir = context.capability.contains(&Capability::SaslIr);

            let mechanism = sasl
                .mechanism
                .or(Some(SaslMechanism::Plain).filter(|_| sasl.plain.is_some()))
                .or(Some(SaslMechanism::Login).filter(|_| sasl.login.is_some()));

            match mechanism {
                None => bail!("no SASL mechanism configured"),
                Some(SaslMechanism::Login) => {
                    let Some(auth) = sasl.login.take() else {
                        bail!("missing SASL LOGIN configuration");
                    };

                    let mut buf = [0u8; READ_BUFFER_SIZE];
                    let mut coroutine = ImapSessionLogin::new(
                        context,
                        ImapSessionLoginParams::new(auth.username, auth.password)?,
                    );
                    let mut arg: Option<&[u8]> = None;

                    context = loop {
                        match coroutine.resume(arg.take()) {
                            ImapSessionLoginResult::Ok(c) => break c,
                            ImapSessionLoginResult::WantsRead => {
                                let n = stream.read(&mut buf)?;
                                arg = Some(&buf[..n]);
                            }
                            ImapSessionLoginResult::WantsWrite(bytes) => {
                                stream.write_all(&bytes)?;
                                arg = None;
                            }
                            ImapSessionLoginResult::Err { err, .. } => bail!(err),
                        }
                    };
                }
                Some(SaslMechanism::Plain) => {
                    let Some(auth) = sasl.plain.take() else {
                        bail!("missing SASL PLAIN configuration");
                    };

                    let mut buf = [0u8; READ_BUFFER_SIZE];
                    let mut coroutine = ImapSessionAuthenticatePlain::new(
                        context,
                        ImapSessionAuthenticatePlainParams::new(
                            auth.authzid,
                            auth.authcid,
                            auth.passwd,
                            ir,
                        ),
                    );
                    let mut arg: Option<&[u8]> = None;

                    context = loop {
                        match coroutine.resume(arg.take()) {
                            ImapSessionAuthenticatePlainResult::Ok(c) => break c,
                            ImapSessionAuthenticatePlainResult::WantsRead => {
                                let n = stream.read(&mut buf)?;
                                arg = Some(&buf[..n]);
                            }
                            ImapSessionAuthenticatePlainResult::WantsWrite(bytes) => {
                                stream.write_all(&bytes)?;
                                arg = None;
                            }
                            ImapSessionAuthenticatePlainResult::Err { err, .. } => bail!(err),
                        }
                    };
                }
                Some(SaslMechanism::Anonymous) => {
                    unimplemented!("ANONYMOUS SASL mechanism not yet implemented")
                }
            }
        }

        Ok(Self { context, stream })
    }
}

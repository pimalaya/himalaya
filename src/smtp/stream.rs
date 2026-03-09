#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::{
    fs,
    io::{self, Read, Write},
    net::TcpStream,
    sync::Arc,
};

use anyhow::{bail, Result};
use gethostname::gethostname;
use io_smtp::{
    context::SmtpContext,
    coroutines::{authenticate::*, ehlo::*, greeting_with_capability::*, starttls::*},
    types::{auth::AuthMechanism, core::EhloDomain, response::Capability, IntoStatic},
};
use io_stream::runtimes::std::handle;
use log::{debug, info};
#[cfg(feature = "native-tls")]
use native_tls::TlsConnector;
#[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
use rustls::{
    crypto::{self, CryptoProvider},
    pki_types::{pem::PemObject, CertificateDer},
    ClientConfig, ClientConnection, StreamOwned,
};
#[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
use rustls_platform_verifier::{ConfigVerifierExt, Verifier};
#[cfg(windows)]
use uds_windows::UnixStream;

use crate::config::{RustlsCryptoConfig, SaslMechanismConfig, SmtpConfig, TlsProviderConfig};

pub enum Stream {
    Tcp(TcpStream),
    Unix(UnixStream),
    #[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
    Rustls(StreamOwned<ClientConnection, TcpStream>),
    #[cfg(feature = "native-tls")]
    NativeTls(native_tls::TlsStream<TcpStream>),
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Tcp(s) => s.read(buf),
            Self::Unix(s) => s.read(buf),
            #[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
            Self::Rustls(s) => s.read(buf),
            #[cfg(feature = "native-tls")]
            Self::NativeTls(s) => s.read(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Tcp(s) => s.write(buf),
            Self::Unix(s) => s.write(buf),
            #[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
            Self::Rustls(s) => s.write(buf),
            #[cfg(feature = "native-tls")]
            Self::NativeTls(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Tcp(s) => s.flush(),
            Self::Unix(s) => s.flush(),
            #[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
            Self::Rustls(s) => s.flush(),
            #[cfg(feature = "native-tls")]
            Self::NativeTls(s) => s.flush(),
        }
    }
}

pub fn connect(mut config: SmtpConfig) -> Result<(SmtpContext, Stream)> {
    info!("connecting to SMTP server using {}", config.url);

    let mut context = SmtpContext::new();
    let host = config.url.host_str().unwrap_or("127.0.0.1");
    let domain = EhloDomain::Domain(gethostname().as_encoded_bytes().try_into()?).into_static();

    let (mut context, mut stream) = match config.url.scheme() {
        scheme if scheme.eq_ignore_ascii_case("smtp") => {
            let port = config.url.port().unwrap_or(25);
            let mut stream = TcpStream::connect((host, port))?;

            let mut coroutine = GetSmtpGreetingWithCapability::new(context, domain.clone());
            let mut arg = None;

            loop {
                match coroutine.resume(arg.take()) {
                    GetSmtpGreetingWithCapabilityResult::Io { io } => {
                        arg = Some(handle(&mut stream, io)?)
                    }
                    GetSmtpGreetingWithCapabilityResult::Ok { context: c } => break context = c,
                    GetSmtpGreetingWithCapabilityResult::Err { err, .. } => Err(err)?,
                }
            }

            (context, Stream::Tcp(stream))
        }
        scheme if scheme.eq_ignore_ascii_case("smtps") => {
            let default_port = if config.starttls { 587 } else { 465 };
            let port = config.url.port().unwrap_or(default_port);
            let mut stream = TcpStream::connect((host, port))?;

            if config.starttls {
                let mut coroutine = SmtpStartTls::new(context);
                let mut arg = None;

                loop {
                    match coroutine.resume(arg.take()) {
                        SmtpStartTlsResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                        SmtpStartTlsResult::Ok { context: c } => break context = c,
                        SmtpStartTlsResult::Err { err, .. } => Err(err)?,
                    }
                }
            }

            let tls_provider = match config.tls.provider {
                #[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
                Some(TlsProviderConfig::Rustls) => TlsProviderConfig::Rustls,
                #[cfg(not(feature = "rustls-aws"))]
                #[cfg(not(feature = "rustls-ring"))]
                Some(TlsProviderConfig::Rustls) => {
                    bail!("Required cargo feature: `rustls-aws` or `rustls-ring`")
                }
                #[cfg(feature = "native-tls")]
                Some(TlsProviderConfig::NativeTls) => TlsProviderConfig::NativeTls,
                #[cfg(not(feature = "native-tls"))]
                Some(TlsProviderConfig::NativeTls) => {
                    bail!("Required cargo feature: `native-tls`")
                }
                #[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
                None => TlsProviderConfig::Rustls,
                #[cfg(not(feature = "rustls-aws"))]
                #[cfg(not(feature = "rustls-ring"))]
                #[cfg(feature = "native-tls")]
                None => TlsProviderConfig::NativeTls,
                #[cfg(not(feature = "rustls-aws"))]
                #[cfg(not(feature = "rustls-ring"))]
                #[cfg(not(feature = "native-tls"))]
                None => {
                    bail!("Required cargo feature: `rustls-aws`, `rustls-ring` or `native-tls`")
                }
            };

            debug!("using TLS provider: {tls_provider:?}");

            let mut stream = match tls_provider {
                #[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
                TlsProviderConfig::Rustls => {
                    let crypto_provider = match config.tls.rustls.crypto {
                        #[cfg(feature = "rustls-aws")]
                        Some(RustlsCryptoConfig::Aws) => RustlsCryptoConfig::Aws,
                        #[cfg(not(feature = "rustls-aws"))]
                        Some(RustlsCryptoConfig::Aws) => {
                            bail!("Required cargo feature: `rustls-aws`");
                        }
                        #[cfg(feature = "rustls-ring")]
                        Some(RustlsCryptoConfig::Ring) => RustlsCryptoConfig::Ring,
                        #[cfg(not(feature = "rustls-ring"))]
                        Some(RustlsCryptoConfig::Ring) => {
                            bail!("Required cargo feature: `rustls-ring`");
                        }
                        #[cfg(feature = "rustls-ring")]
                        None => RustlsCryptoConfig::Ring,
                        #[cfg(not(feature = "rustls-ring"))]
                        #[cfg(feature = "rustls-aws")]
                        None => RustlsCryptoConfig::Aws,
                        #[cfg(not(feature = "rustls-aws"))]
                        #[cfg(not(feature = "rustls-ring"))]
                        None => {
                            bail!("Required cargo feature: `rustls-aws` or `rustls-ring`");
                        }
                    };

                    debug!("using rustls crypto provider: {crypto_provider:?}");

                    let crypto_provider = match crypto_provider {
                        #[cfg(feature = "rustls-aws")]
                        RustlsCryptoConfig::Aws => crypto::aws_lc_rs::default_provider(),
                        #[cfg(feature = "rustls-ring")]
                        RustlsCryptoConfig::Ring => crypto::ring::default_provider(),
                        #[allow(unreachable_patterns)]
                        _ => unreachable!(),
                    };

                    let crypto_provider = match crypto_provider.install_default() {
                        Ok(()) => CryptoProvider::get_default().unwrap().clone(),
                        Err(crypto_provider) => crypto_provider,
                    };

                    let mut config = if let Some(pem_path) = &config.tls.cert {
                        debug!("using TLS cert at {}", pem_path.display());
                        let pem = fs::read(pem_path)?;

                        let Some(cert) = CertificateDer::pem_slice_iter(&pem).next() else {
                            bail!("empty TLS cert at {}", pem_path.display())
                        };

                        let verifier =
                            Verifier::new_with_extra_roots(vec![cert?], crypto_provider)?;

                        ClientConfig::builder()
                            .dangerous()
                            .with_custom_certificate_verifier(Arc::new(verifier))
                            .with_no_client_auth()
                    } else {
                        debug!("using OS TLS certs");
                        ClientConfig::with_platform_verifier()?
                    };

                    config.alpn_protocols = vec![b"smtp".to_vec()];

                    let server_name = host.to_string().try_into()?;
                    let conn = ClientConnection::new(Arc::new(config), server_name)?;
                    Stream::Rustls(StreamOwned::new(conn, stream))
                }
                #[cfg(feature = "native-tls")]
                TlsProviderConfig::NativeTls => {
                    let mut builder = TlsConnector::builder();

                    if let Some(pem_path) = &config.tls.cert {
                        debug!("using TLS cert at {}", pem_path.display());
                        let pem = fs::read(pem_path)?;
                        let cert = native_tls::Certificate::from_pem(&pem)?;
                        builder.add_root_certificate(cert);
                    }

                    let connector = builder.build()?;
                    Stream::NativeTls(connector.connect(host, stream)?)
                }
                #[allow(unreachable_patterns)]
                _ => unreachable!(),
            };

            if config.starttls {
                let mut coroutine = SmtpEhlo::new(context, domain.clone());
                let mut arg = None;

                loop {
                    match coroutine.resume(arg.take()) {
                        SmtpEhloResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                        SmtpEhloResult::Ok { context: c } => break context = c,
                        SmtpEhloResult::Err { err, .. } => Err(err)?,
                    }
                }
            } else {
                let mut coroutine = GetSmtpGreetingWithCapability::new(context, domain.clone());
                let mut arg = None;

                loop {
                    match coroutine.resume(arg.take()) {
                        GetSmtpGreetingWithCapabilityResult::Io { io } => {
                            arg = Some(handle(&mut stream, io)?)
                        }
                        GetSmtpGreetingWithCapabilityResult::Ok { context: c } => {
                            break context = c
                        }
                        GetSmtpGreetingWithCapabilityResult::Err { err, .. } => Err(err)?,
                    }
                }
            }

            (context, stream)
        }
        scheme if scheme.eq_ignore_ascii_case("unix") => {
            let sock_path = config.url.path();
            let mut stream = UnixStream::connect(&sock_path)?;

            let mut coroutine = GetSmtpGreetingWithCapability::new(context, domain.clone());
            let mut arg = None;

            loop {
                match coroutine.resume(arg.take()) {
                    GetSmtpGreetingWithCapabilityResult::Io { io } => {
                        arg = Some(handle(&mut stream, io)?)
                    }
                    GetSmtpGreetingWithCapabilityResult::Ok { context: c } => break context = c,
                    GetSmtpGreetingWithCapabilityResult::Err { err, .. } => Err(err)?,
                }
            }

            (context, Stream::Unix(stream))
        }
        scheme => {
            bail!("Unknown scheme {scheme}, expected smtp, smtps or unix");
        }
    };

    if !context.authenticated {
        let mut candidates = vec![];

        for mechanism in config.sasl.mechanisms {
            match mechanism {
                SaslMechanismConfig::Login => {
                    let Some(auth) = config.sasl.login.take() else {
                        debug!("missing SASL LOGIN configuration, skipping it");
                        continue;
                    };

                    for capability in &context.capability {
                        match capability {
                            Capability::Auth(mechanisms) => {
                                for m in mechanisms {
                                    match m {
                                        AuthMechanism::Login => {
                                            candidates.push(SmtpAuthenticateCandidate::Login {
                                                login: auth.username.clone(),
                                                password: auth.password.get()?,
                                                domain: domain.clone(),
                                            });
                                            break;
                                        }
                                        _ => continue,
                                    }
                                }
                            }
                            _ => continue,
                        }
                    }

                    debug!("SASL LOGIN disabled by the server, skipping it");
                    continue;
                }
                SaslMechanismConfig::Plain => {
                    let Some(auth) = config.sasl.plain.take() else {
                        debug!("missing SASL PLAIN configuration, skipping it");
                        continue;
                    };

                    for capability in &context.capability {
                        match capability {
                            Capability::Auth(mechanisms) => {
                                for m in mechanisms {
                                    match m {
                                        AuthMechanism::Plain => {
                                            candidates.push(SmtpAuthenticateCandidate::Plain {
                                                login: auth.authcid.clone(),
                                                password: auth.passwd.get()?,
                                                domain: domain.clone(),
                                            });
                                            break;
                                        }
                                        _ => continue,
                                    }
                                }
                            }
                            _ => continue,
                        }
                    }

                    debug!("SASL PLAIN disabled by the server, skipping it");
                    continue;
                }
                SaslMechanismConfig::Anonymous => {
                    unimplemented!("ANONYMOUS SASL mechanism not yet implemented")
                }
            };
        }

        let mut arg = None;
        let mut coroutine = SmtpAuthenticate::new(context, candidates);

        loop {
            match coroutine.resume(arg.take()) {
                SmtpAuthenticateResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                SmtpAuthenticateResult::Ok { context: c, .. } => break context = c,
                SmtpAuthenticateResult::Err { err, .. } => bail!(err),
            }
        }
    }

    Ok((context, stream))
}

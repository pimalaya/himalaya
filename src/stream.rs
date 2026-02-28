#[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
use std::{fs, path::PathBuf, sync::Arc};
use std::{
    io::{self, Read, Write},
    net::TcpStream,
};

#[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
use anyhow::bail;
use anyhow::Result;
use io_stream::runtimes::std::handle;
#[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
use rustls::{
    pki_types::{pem::PemObject, CertificateDer},
    ClientConfig, ClientConnection, StreamOwned,
};
#[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
use rustls_platform_verifier::{ConfigVerifierExt, Verifier};

#[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
use io_imap::coroutines::starttls::*;
use io_imap::{
    context::ImapContext,
    coroutines::{capability::*, greeting_with_capability::*},
};

/// Creates an insecure client, using TCP.
///
/// This constructor creates a client based on an raw
/// [`TcpStream`], receives greeting then saves server
/// capabilities.
pub fn tcp(host: impl AsRef<str>, port: u16) -> Result<(ImapContext, Stream)> {
    let mut context = ImapContext::new();
    let mut tcp = TcpStream::connect((host.as_ref(), port))?;

    let mut coroutine = GetImapGreetingWithCapability::new(context);
    let mut arg = None;

    loop {
        match coroutine.resume(arg.take()) {
            GetImapGreetingWithCapabilityResult::Ok(out) => break context = out.context,
            GetImapGreetingWithCapabilityResult::Io(io) => {
                arg = Some(handle(&mut tcp, io).unwrap())
            }
            GetImapGreetingWithCapabilityResult::Err(err) => Err(err)?,
        }
    }

    Ok((context, Stream::Tcp(tcp)))
}

#[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
pub fn rustls(
    host: impl ToString,
    port: u16,
    starttls: bool,
    cert: Option<PathBuf>,
) -> Result<(ImapContext, Stream)> {
    let host = host.to_string();
    let mut context = ImapContext::new();
    let mut tcp = TcpStream::connect((host.as_str(), port))?;

    if starttls {
        let mut coroutine = ImapStartTls::new(context);
        let mut arg = None;

        loop {
            match coroutine.resume(arg.take()) {
                ImapStartTlsResult::Ok(out) => break context = out.context,
                ImapStartTlsResult::Io(io) => arg = Some(handle(&mut tcp, io)?),
                ImapStartTlsResult::Err(err) => Err(err)?,
            }
        }
    }

    let mut config = if let Some(pem_path) = cert {
        let pem = fs::read(&pem_path)?;

        let Some(cert) = CertificateDer::pem_slice_iter(&pem).next() else {
            bail!("empty cert at {}", pem_path.display())
        };

        let verifier = Verifier::new_with_extra_roots(vec![cert?])?;

        ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(verifier))
            .with_no_client_auth()
    } else {
        ClientConfig::with_platform_verifier()
    };

    // See <https://www.iana.org/assignments/tls-extensiontype-values/tls-extensiontype-values.xhtml#alpn-protocol-ids>
    config.alpn_protocols = vec![b"imap".to_vec()];

    let server_name = host.try_into()?;
    let conn = ClientConnection::new(Arc::new(config), server_name)?;
    let mut tls = StreamOwned::new(conn, tcp);

    if starttls {
        let mut coroutine = GetImapCapability::new(context);
        let mut arg = None;

        loop {
            match coroutine.resume(arg.take()) {
                GetImapCapabilityResult::Ok { context: c } => break context = c,
                GetImapCapabilityResult::Io(io) => arg = Some(handle(&mut tls, io)?),
                GetImapCapabilityResult::Err { err, .. } => Err(err)?,
            }
        }
    } else {
        let mut coroutine = GetImapGreetingWithCapability::new(context);
        let mut arg = None;

        loop {
            match coroutine.resume(arg.take()) {
                GetImapGreetingWithCapabilityResult::Ok(out) => break context = out.context,
                GetImapGreetingWithCapabilityResult::Io(io) => arg = Some(handle(&mut tls, io)?),
                GetImapGreetingWithCapabilityResult::Err(err) => Err(err)?,
            }
        }
    };

    Ok((context, Stream::Rustls(tls)))
}

pub enum Stream {
    Tcp(TcpStream),
    #[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
    Rustls(StreamOwned<ClientConnection, TcpStream>),
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Stream::Tcp(stream) => stream.read(buf),
            #[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
            Stream::Rustls(stream) => stream.read(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Stream::Tcp(stream) => stream.write(buf),
            #[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
            Stream::Rustls(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Stream::Tcp(stream) => stream.flush(),
            #[cfg(any(feature = "rustls-aws", feature = "rustls-ring"))]
            Stream::Rustls(stream) => stream.flush(),
        }
    }
}

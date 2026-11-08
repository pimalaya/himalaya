//! # Typed server URL
//!
//! A server the user typed, when discovery found nothing or a custom one
//! was asked for. The scheme names the protocol and its transport, a bare
//! host taking the protocol's default port.
//!
//! Encrypted servers only: `http://` is refused rather than sending a
//! password in clear, unless its host is this machine, and `imap://` and
//! `smtp://` mean STARTTLS.

use std::{error, fmt, net::IpAddr};

use percent_encoding::percent_decode_str;
use url::Url;

use crate::wizard::plan::{Domain, Endpoint, Protocol, Route, SignIn};

/// A typed server, before its sign-in method is picked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedServer {
    /// The protocol the scheme names.
    pub protocol: Protocol,
    /// The server.
    pub endpoint: Endpoint,
    /// The user name the URL carried.
    pub login: Option<String>,
}

/// Why a typed server URL is refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerUrlError {
    /// No `scheme://` prefix.
    MissingScheme,
    /// A cleartext URL, as typed.
    Cleartext(String),
    /// A scheme the domain does not take.
    Unsupported { scheme: String, domain: Domain },
    /// A password written in the URL.
    Password,
    /// No host after the scheme.
    MissingHost,
    /// Not a URL at all.
    Invalid(String),
}

impl TypedServer {
    /// Reads a server URL typed for `domain`.
    pub fn parse(input: &str, domain: Domain) -> Result<Self, ServerUrlError> {
        let input = input.trim();

        let Some((scheme, _)) = input.split_once("://") else {
            return Err(ServerUrlError::MissingScheme);
        };

        let scheme = scheme.to_ascii_lowercase();
        let url = Url::parse(input).map_err(|err| ServerUrlError::Invalid(err.to_string()))?;

        if url.password().is_some() {
            return Err(ServerUrlError::Password);
        }

        let host = match url.host_str() {
            Some(host) if !host.is_empty() => host.trim_end_matches('.').to_lowercase(),
            _ => return Err(ServerUrlError::MissingHost),
        };

        let local = is_loopback(&host);

        let (protocol, transport) = match (scheme.as_str(), domain) {
            ("http", _) if !local => return Err(ServerUrlError::Cleartext(input.to_string())),
            ("http", Domain::Mail) => (Protocol::Jmap, Transport::Web("http")),
            ("imaps", Domain::Mail) => (
                Protocol::Imap,
                Transport::Tcp {
                    tls: true,
                    port: 993,
                },
            ),
            ("imap", Domain::Mail) => (
                Protocol::Imap,
                Transport::Tcp {
                    tls: false,
                    port: 143,
                },
            ),
            ("https" | "jmaps" | "jmap", Domain::Mail) => (Protocol::Jmap, Transport::Web("https")),
            ("smtps", Domain::Sending) => (
                Protocol::Smtp,
                Transport::Tcp {
                    tls: true,
                    port: 465,
                },
            ),
            ("smtp", Domain::Sending) => (
                Protocol::Smtp,
                Transport::Tcp {
                    tls: false,
                    port: 587,
                },
            ),
            _ => return Err(ServerUrlError::Unsupported { scheme, domain }),
        };

        let login = Some(
            percent_decode_str(url.username())
                .decode_utf8_lossy()
                .into_owned(),
        )
        .filter(|login| !login.is_empty());

        let endpoint = match transport {
            Transport::Tcp { tls, port } => Endpoint::Tcp {
                host,
                port: url.port().unwrap_or(port),
                tls,
            },
            Transport::Web(web) => {
                let port = url
                    .port()
                    .map(|port| format!(":{port}"))
                    .unwrap_or_default();
                let web = format!("{web}://{host}{port}{}", url.path());
                let web =
                    Url::parse(&web).map_err(|err| ServerUrlError::Invalid(err.to_string()))?;
                Endpoint::Http(web.to_string())
            }
        };

        Ok(Self {
            protocol,
            endpoint,
            login,
        })
    }

    /// The route signing in to the server with `sign_in`.
    pub fn route(self, sign_in: SignIn) -> Route {
        Route {
            protocol: self.protocol,
            endpoint: self.endpoint,
            sign_in,
            login: self.login,
            authority: None,
        }
    }
}

impl fmt::Display for ServerUrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingScheme => write!(
                f,
                "Missing scheme, as in imaps://imap.example.org or smtps://smtp.example.org"
            ),
            Self::Cleartext(url) => {
                write!(f, "Refused {url}: only encrypted servers are accepted")
            }
            Self::Unsupported {
                scheme,
                domain: Domain::Mail,
            } => write!(
                f,
                "Unsupported scheme `{scheme}` for mail: \
                 use imaps://, imap://, https://, jmaps:// or jmap://"
            ),
            Self::Unsupported {
                scheme,
                domain: Domain::Sending,
            } => write!(
                f,
                "Unsupported scheme `{scheme}` for sending: use smtps:// or smtp://"
            ),
            Self::Password => write!(f, "Type the password when asked rather than in the URL"),
            Self::MissingHost => write!(f, "Missing host in the server URL"),
            Self::Invalid(err) => write!(f, "Invalid server URL: {err}"),
        }
    }
}

impl error::Error for ServerUrlError {}

/// How a scheme reaches its server.
enum Transport {
    /// TCP, over implicit TLS or STARTTLS, on this port by default.
    Tcp { tls: bool, port: u16 },
    /// HTTP, under this scheme.
    Web(&'static str),
}

/// Whether `host` is this machine: `localhost` or a loopback address.
fn is_loopback(host: &str) -> bool {
    let bare = host.trim_start_matches('[').trim_end_matches(']');

    bare == "localhost" || bare.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

#[cfg(test)]
mod tests {
    use crate::wizard::{
        plan::{Domain, Endpoint, Protocol},
        server::*,
    };

    fn tcp(input: &str, domain: Domain) -> (Protocol, String, u16, bool) {
        let server = TypedServer::parse(input, domain).expect("a valid server URL");

        match server.endpoint {
            Endpoint::Tcp { host, port, tls } => (server.protocol, host, port, tls),
            Endpoint::Http(url) => panic!("expected a TCP server, got {url}"),
        }
    }

    fn http(input: &str) -> String {
        match TypedServer::parse(input, Domain::Mail).map(|server| server.endpoint) {
            Ok(Endpoint::Http(url)) => url,
            other => panic!("expected an https URL, got {other:?}"),
        }
    }

    #[test]
    fn a_scheme_names_the_protocol_and_its_default_port() {
        assert_eq!(
            tcp("imaps://mail.example.org", Domain::Mail),
            (Protocol::Imap, String::from("mail.example.org"), 993, true)
        );
        assert_eq!(
            tcp("imap://Mail.Example.org", Domain::Mail),
            (Protocol::Imap, String::from("mail.example.org"), 143, false)
        );
        assert_eq!(
            tcp("smtps://mail.example.org", Domain::Sending),
            (Protocol::Smtp, String::from("mail.example.org"), 465, true)
        );
        assert_eq!(
            tcp("smtp://mail.example.org", Domain::Sending),
            (Protocol::Smtp, String::from("mail.example.org"), 587, false)
        );
        assert_eq!(
            tcp("IMAPS://mail.example.org:1993", Domain::Mail),
            (Protocol::Imap, String::from("mail.example.org"), 1993, true)
        );
    }

    #[test]
    fn jmap_schemes_read_as_https() {
        assert_eq!(http("https://api.example.org"), "https://api.example.org/");
        assert_eq!(http("jmap://api.example.org"), "https://api.example.org/");
        assert_eq!(
            http("jmaps://api.example.org:8443/jmap/session"),
            "https://api.example.org:8443/jmap/session"
        );
        assert_eq!(
            TypedServer::parse("https://api.example.org", Domain::Mail)
                .map(|server| server.protocol),
            Ok(Protocol::Jmap)
        );
    }

    #[test]
    fn a_cleartext_server_is_refused_by_name() {
        let err = TypedServer::parse("http://mail.example.org", Domain::Mail).unwrap_err();

        assert_eq!(
            err,
            ServerUrlError::Cleartext(String::from("http://mail.example.org"))
        );
        assert_eq!(
            err.to_string(),
            "Refused http://mail.example.org: only encrypted servers are accepted"
        );
        assert_eq!(
            TypedServer::parse("http://smtp.example.org", Domain::Sending),
            Err(ServerUrlError::Cleartext(String::from(
                "http://smtp.example.org"
            )))
        );
        assert_eq!(
            TypedServer::parse("http://localhost.example.org", Domain::Mail),
            Err(ServerUrlError::Cleartext(String::from(
                "http://localhost.example.org"
            )))
        );
    }

    #[test]
    fn a_cleartext_server_on_this_machine_is_taken() {
        assert_eq!(
            http("http://localhost:8080/jmap/session"),
            "http://localhost:8080/jmap/session"
        );
        assert_eq!(http("http://127.0.0.2"), "http://127.0.0.2/");
        assert_eq!(http("http://[::1]:8080"), "http://[::1]:8080/");

        // A loopback host does not make HTTP a sending protocol.
        assert_eq!(
            TypedServer::parse("http://localhost", Domain::Sending),
            Err(ServerUrlError::Unsupported {
                scheme: String::from("http"),
                domain: Domain::Sending,
            })
        );
    }

    #[test]
    fn a_domain_takes_its_own_schemes_only() {
        assert_eq!(
            TypedServer::parse("smtps://smtp.example.org", Domain::Mail),
            Err(ServerUrlError::Unsupported {
                scheme: String::from("smtps"),
                domain: Domain::Mail,
            })
        );
        assert_eq!(
            TypedServer::parse("imaps://imap.example.org", Domain::Sending),
            Err(ServerUrlError::Unsupported {
                scheme: String::from("imaps"),
                domain: Domain::Sending,
            })
        );
        assert_eq!(
            TypedServer::parse("pop3s://pop.example.org", Domain::Mail),
            Err(ServerUrlError::Unsupported {
                scheme: String::from("pop3s"),
                domain: Domain::Mail,
            })
        );
    }

    #[test]
    fn a_malformed_url_is_refused() {
        assert_eq!(
            TypedServer::parse("mail.example.org", Domain::Mail),
            Err(ServerUrlError::MissingScheme)
        );
        assert_eq!(
            TypedServer::parse("imaps://", Domain::Mail),
            Err(ServerUrlError::MissingHost)
        );
        assert_eq!(
            TypedServer::parse("imaps://jane:secret@imap.example.org", Domain::Mail),
            Err(ServerUrlError::Password)
        );
    }

    #[test]
    fn a_user_name_is_the_login() {
        let server =
            TypedServer::parse("imaps://jane%40example.org@imap.example.org", Domain::Mail)
                .expect("a valid server URL");

        assert_eq!(server.login.as_deref(), Some("jane@example.org"));
        assert_eq!(
            TypedServer::parse("imaps://imap.example.org", Domain::Mail).map(|server| server.login),
            Ok(None)
        );
    }
}

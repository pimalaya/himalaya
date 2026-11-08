//! # IMAP wizard
//!
//! Builds and tests the IMAP block of a route the review settled.
//!
//! The SASL mechanism is never asked: an unauthenticated CAPABILITY probe
//! says what the server takes, and the sign-in method picks the best of
//! it. The block stays editable afterwards.

use anyhow::{Result, bail};
use io_imap::client::default_alpn;
use io_sasl::mechanism::SaslMechanism;
use pimalaya_config::secret::{Secret, SecretResolver};

use crate::{
    account::check,
    config::{
        ImapConfig, SaslConfig, SaslLoginConfig, SaslOauthbearerConfig, SaslPlainConfig,
        SaslScramSha256Config, SaslXoauth2Config,
    },
    wizard::plan::{Endpoint, SignIn},
};

/// Signs `login` in to the IMAP server at `endpoint` with `secret`,
/// answering the block that connected.
pub fn connect(
    endpoint: &Endpoint,
    sign_in: SignIn,
    login: &str,
    secret: Secret,
) -> Result<ImapConfig> {
    let Endpoint::Tcp { host, port, tls } = endpoint else {
        bail!("Expected an IMAP server, got {endpoint:?}");
    };

    let scheme = if *tls { "imaps" } else { "imap" };
    let server = format!("{scheme}://{host}:{port}");

    let probed = match check::probe_imap_mechanisms(&server, !tls) {
        Ok(mechanisms) => Some(mechanisms),
        Err(err) => {
            log::warn!("cannot probe IMAP capabilities, keeping the default mechanism: {err:#}");
            None
        }
    };

    let mechanism = mechanism(sign_in, probed.as_deref());

    let config = ImapConfig {
        server,
        tls: Default::default(),
        proxy: None,
        starttls: !tls,
        alpn: default_alpn(),
        sasl: Some(sasl(mechanism, login, secret)),
        sasl_ir: None,
        id: Default::default(),
        sort: Default::default(),
    };

    check::connect_imap(&config, &mut SecretResolver::new())?;

    Ok(config)
}

/// The mechanism a sign-in takes among the `probed` ones: SCRAM-SHA-256,
/// PLAIN, then the LOGIN command for a password, OAUTHBEARER for a token;
/// PLAIN and XOAUTH2 when the probe failed or offered none of them.
fn mechanism(sign_in: SignIn, probed: Option<&[SaslMechanism]>) -> SaslMechanism {
    let probed = probed.unwrap_or_default();

    let (preferred, fallback): (&[SaslMechanism], SaslMechanism) = match sign_in {
        SignIn::Password => (
            &[
                SaslMechanism::ScramSha256,
                SaslMechanism::Plain,
                SaslMechanism::Login,
            ],
            SaslMechanism::Plain,
        ),
        SignIn::Token | SignIn::Oauth => (&[SaslMechanism::OAuthBearer], SaslMechanism::XOAuth2),
    };

    preferred
        .iter()
        .find(|mechanism| probed.contains(mechanism))
        .copied()
        .unwrap_or(fallback)
}

/// The SASL block signing `login` in with `secret` over `mechanism`.
fn sasl(mechanism: SaslMechanism, login: &str, secret: Secret) -> SaslConfig {
    let login = login.to_string();

    match mechanism {
        SaslMechanism::ScramSha256 => SaslConfig::ScramSha256(SaslScramSha256Config {
            username: login,
            password: secret,
        }),
        SaslMechanism::Login => SaslConfig::Login(SaslLoginConfig {
            username: login,
            password: secret,
        }),
        SaslMechanism::OAuthBearer => SaslConfig::Oauthbearer(SaslOauthbearerConfig {
            username: login,
            token: secret,
        }),
        SaslMechanism::XOAuth2 => SaslConfig::Xoauth2(SaslXoauth2Config {
            username: login,
            token: secret,
        }),
        _ => SaslConfig::Plain(SaslPlainConfig {
            authzid: None,
            authcid: login,
            passwd: secret,
        }),
    }
}

#[cfg(test)]
mod tests {
    use io_sasl::mechanism::SaslMechanism;

    use crate::wizard::{imap::*, plan::SignIn};

    #[test]
    fn a_password_takes_the_best_probed_mechanism() {
        let all = [
            SaslMechanism::ScramSha256,
            SaslMechanism::Plain,
            SaslMechanism::OAuthBearer,
            SaslMechanism::XOAuth2,
            SaslMechanism::Login,
        ];

        assert_eq!(
            mechanism(SignIn::Password, Some(&all)),
            SaslMechanism::ScramSha256
        );
        assert_eq!(
            mechanism(
                SignIn::Password,
                Some(&[SaslMechanism::Plain, SaslMechanism::Login])
            ),
            SaslMechanism::Plain
        );
        assert_eq!(
            mechanism(SignIn::Password, Some(&[SaslMechanism::Login])),
            SaslMechanism::Login
        );
        assert_eq!(mechanism(SignIn::Password, None), SaslMechanism::Plain);
        assert_eq!(
            mechanism(SignIn::Password, Some(&[SaslMechanism::XOAuth2])),
            SaslMechanism::Plain
        );
    }

    #[test]
    fn a_token_takes_oauthbearer_when_advertised() {
        assert_eq!(
            mechanism(
                SignIn::Oauth,
                Some(&[SaslMechanism::OAuthBearer, SaslMechanism::XOAuth2])
            ),
            SaslMechanism::OAuthBearer
        );
        assert_eq!(
            mechanism(SignIn::Token, Some(&[SaslMechanism::XOAuth2])),
            SaslMechanism::XOAuth2
        );
        assert_eq!(mechanism(SignIn::Oauth, None), SaslMechanism::XOAuth2);
    }
}

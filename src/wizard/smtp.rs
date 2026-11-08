//! # SMTP wizard
//!
//! Builds and tests the SMTP block of a route the review settled.
//!
//! SMTP negotiates its mechanism over EHLO rather than through a probe
//! the wizard can run first, so a password takes PLAIN and a token
//! XOAUTH2, the two every submission server taking them offers. The block
//! stays editable afterwards.

use anyhow::{Result, bail};
use io_smtp::client::SmtpClientStd;
use pimalaya_config::secret::{Secret, SecretResolver};

use crate::{
    account::check,
    config::{SaslConfig, SaslPlainConfig, SaslXoauth2Config, SmtpConfig},
    wizard::plan::{Endpoint, SignIn},
};

/// Signs `login` in to the SMTP server at `endpoint` with `secret`,
/// answering the block that connected.
pub fn connect(
    endpoint: &Endpoint,
    sign_in: SignIn,
    login: &str,
    secret: Secret,
) -> Result<SmtpConfig> {
    let Endpoint::Tcp { host, port, tls } = endpoint else {
        bail!("Expected an SMTP server, got {endpoint:?}");
    };

    let scheme = if *tls { "smtps" } else { "smtp" };

    let sasl = match sign_in {
        SignIn::Password => SaslConfig::Plain(SaslPlainConfig {
            authzid: None,
            authcid: login.to_string(),
            passwd: secret,
        }),
        SignIn::Token | SignIn::Oauth => SaslConfig::Xoauth2(SaslXoauth2Config {
            username: login.to_string(),
            token: secret,
        }),
    };

    let config = SmtpConfig {
        server: format!("{scheme}://{host}:{port}"),
        tls: Default::default(),
        proxy: None,
        starttls: !tls,
        alpn: SmtpClientStd::default_alpn(),
        sasl: Some(sasl),
    };

    check::connect_smtp(&config, &mut SecretResolver::new())?;

    Ok(config)
}

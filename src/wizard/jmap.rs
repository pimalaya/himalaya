//! # JMAP wizard
//!
//! Builds and tests the JMAP block of a route the review settled: Basic
//! for a password, Bearer for a token, never asked.

use anyhow::{Result, bail};
use io_jmap::client::JmapClientStd;
use pimalaya_config::secret::Secret;

use crate::{
    config::{JmapAuthConfig, JmapConfig},
    jmap::client::JmapClient,
    wizard::plan::{Endpoint, SignIn},
};

/// Signs in to the JMAP session at `endpoint` with `secret`, `login`
/// naming the account of a password, answering the block that connected.
pub fn connect(
    endpoint: &Endpoint,
    sign_in: SignIn,
    login: &str,
    secret: Secret,
) -> Result<JmapConfig> {
    let Endpoint::Http(server) = endpoint else {
        bail!("Expected a JMAP session URL, got {endpoint:?}");
    };

    let auth = match sign_in {
        SignIn::Password => JmapAuthConfig::Basic {
            username: login.to_string(),
            password: secret,
        },
        SignIn::Token | SignIn::Oauth => JmapAuthConfig::Bearer { token: secret },
    };

    let config = JmapConfig {
        server: server.clone(),
        tls: Default::default(),
        proxy: None,
        alpn: JmapClientStd::default_alpn(),
        auth,
        identity_id: None,
        drafts_mailbox_id: None,
        sent_mailbox_id: None,
    };

    JmapClient::new(config.clone())?;

    Ok(config)
}

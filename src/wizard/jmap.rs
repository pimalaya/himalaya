//! JMAP wizard.
//!
//! A discovery entry pins the session endpoint and the authentication
//! method, so [`configure_discovered`] prompts only the credentials.
//! [`configure_manual`] handles a typed server URL: it prompts the
//! authentication strategy too.

use anyhow::{Result, bail};
use pimalaya_cli::prompt;

use crate::{
    config::{JmapAuthConfig, JmapConfig},
    wizard::{
        search::{Discovered, DiscoveredAuth, DiscoveredKind},
        secret,
    },
};

const BASIC: &str = "Basic (login + password)";
const BEARER: &str = "Bearer (API token)";
const AUTHS: [&str; 2] = [BASIC, BEARER];

/// Configures JMAP from a discovered entry: the endpoint and the
/// authentication method are pinned, only the credentials are prompted.
pub fn configure_discovered(email: &str, discovered: &Discovered) -> Result<JmapConfig> {
    let DiscoveredKind::Jmap(server) = &discovered.kind else {
        bail!("Expected a JMAP configuration");
    };

    let auth = match discovered.auth {
        DiscoveredAuth::Password => {
            let default_login = discovered.login_default(email);
            let username = prompt::text("Login:", default_login.as_deref())?;
            let password = secret::configure("JMAP password", None)?;
            JmapAuthConfig::Basic { username, password }
        }
        DiscoveredAuth::Token => {
            let token = secret::configure("JMAP API token", Some("ortie token show"))?;
            JmapAuthConfig::Bearer { token }
        }
        DiscoveredAuth::OAuth => bail!("OAuth 2.0 cannot be configured directly"),
    };

    Ok(jmap_config(server.clone(), auth))
}

/// Configures JMAP against a typed `server` URL, prompting the
/// authentication strategy and credentials.
pub fn configure_manual(server: &str, login_hint: Option<&str>) -> Result<JmapConfig> {
    let strategy = prompt::item("JMAP authentication:", AUTHS, None)?;

    let auth = match strategy {
        BASIC => {
            let username = prompt::text("Login:", login_hint)?;
            let password = secret::configure("JMAP password", None)?;
            JmapAuthConfig::Basic { username, password }
        }
        BEARER => {
            let token = secret::configure("JMAP API token", Some("ortie token show"))?;
            JmapAuthConfig::Bearer { token }
        }
        _ => unreachable!(),
    };

    Ok(jmap_config(server.to_string(), auth))
}

fn jmap_config(server: String, auth: JmapAuthConfig) -> JmapConfig {
    JmapConfig {
        server,
        tls: Default::default(),
        alpn: io_jmap::client::JmapClientStd::default_alpn(),
        auth,
        identity_id: None,
        drafts_mailbox_id: None,
    }
}

//! JMAP wizard.
//!
//! A discovery entry pins the session endpoint and the authentication
//! method, so [`configure_discovered`] prompts only the credentials.

use std::collections::HashMap;

use anyhow::{Result, bail};
use pimalaya_cli::{prompt, spinner::Spinner};

use crate::{
    config::{JmapAuthConfig, JmapConfig},
    jmap::client::JmapClient,
    wizard::{
        mailbox,
        search::{AuthCaps, Discovered, DiscoveredKind},
        secret,
    },
};

const BASIC: &str = "Basic (username + password)";
const BEARER: &str = "Bearer (API token)";

/// Configures JMAP from a discovered entry: the endpoint is pinned, the
/// HTTP authentication scheme is picked among the advertised ones (skipped
/// when only one qualifies), then its credentials are prompted. The
/// connection is tested and its role-based `mailbox.alias.*` entries
/// discovered on the same session, so the caller skips the final account
/// test.
pub fn configure_discovered(
    account_name: &str,
    email: &str,
    discovered: &Discovered,
) -> Result<(JmapConfig, HashMap<String, String>)> {
    let DiscoveredKind::Jmap(server) = &discovered.kind else {
        bail!("Expected a JMAP configuration");
    };

    let auth = prompt_auth(
        account_name,
        discovered.login_default(email).as_deref(),
        discovered.auth,
    )?;

    let config = jmap_config(server.clone(), auth);
    let aliases = test_and_discover(&config)?;

    Ok((config, aliases))
}

/// Connects to JMAP — the connection test — and best-effort discovers the
/// role-based mailbox aliases on the same session. A failed connection is
/// the wizard's error; a failed listing only means fewer aliases.
fn test_and_discover(config: &JmapConfig) -> Result<HashMap<String, String>> {
    let spinner = Spinner::start("Testing JMAP connection");

    let mut client = match JmapClient::new(config.clone()) {
        Ok(client) => client,
        Err(err) => {
            spinner.failure("JMAP connection failed");
            return Err(err);
        }
    };

    spinner.success("JMAP connection succeeded");
    Ok(mailbox::jmap_aliases(&mut client))
}

/// Prompts the HTTP authentication scheme from `caps` (both offered when
/// none was advertised), then its credentials. The Bearer token flow shows
/// the OAuth brokers only when a grant was advertised.
fn prompt_auth(
    account_name: &str,
    login_hint: Option<&str>,
    caps: AuthCaps,
) -> Result<JmapAuthConfig> {
    let mut schemes = Vec::new();
    if caps.basic || !caps.any() {
        schemes.push(BASIC);
    }
    if caps.token() || !caps.any() {
        schemes.push(BEARER);
    }

    let scheme = if schemes.len() == 1 {
        schemes[0]
    } else {
        prompt::item("JMAP authentication:", schemes, None)?
    };

    let key = format!("{account_name}-jmap");
    Ok(match scheme {
        BASIC => {
            let username = prompt::text("Login:", login_hint)?;
            let password = secret::configure_password("JMAP password", &key)?;
            JmapAuthConfig::Basic { username, password }
        }
        BEARER => {
            let token = secret::configure_token("JMAP API token", &key, caps.oauth || !caps.any())?;
            JmapAuthConfig::Bearer { token }
        }
        _ => unreachable!(),
    })
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

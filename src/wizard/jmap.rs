//! # JMAP wizard
//!
//! Configures an account against the session endpoint discovery pinned,
//! which also names the authentication method, so only the credentials
//! are prompted.

use anyhow::{Result, bail};
use io_jmap::client::JmapClientStd;
use pimalaya_cli::{prompt, spinner::Spinner};

use crate::{
    config::{JmapAuthConfig, JmapConfig},
    jmap::client::JmapClient,
    wizard::{
        search::{AuthCaps, Discovered, DiscoveredKind},
        secret,
    },
};

const BASIC: &str = "Basic (username + password)";
const BEARER: &str = "Bearer (API token)";

/// Configures JMAP from a discovered entry, whose endpoint is pinned.
///
/// The HTTP scheme is picked among the advertised ones, skipped when only
/// one qualifies, and its credentials prompted. The connection is tested,
/// so the caller skips the final account test.
pub fn configure_discovered(
    account_name: &str,
    email: &str,
    discovered: &Discovered,
) -> Result<JmapConfig> {
    let DiscoveredKind::Jmap(server) = &discovered.kind else {
        bail!("Expected a JMAP configuration");
    };

    let auth = prompt_auth(
        account_name,
        discovered.login_default(email).as_deref(),
        discovered.auth,
    )?;

    let config = jmap_config(server.clone(), auth);
    test_connection(&config)?;

    Ok(config)
}

/// Connects to JMAP, which is the connection test.
fn test_connection(config: &JmapConfig) -> Result<()> {
    let spinner = Spinner::start("Testing JMAP connection");

    match JmapClient::new(config.clone()) {
        Ok(_) => {
            spinner.success("JMAP connection succeeded");
            Ok(())
        }
        Err(err) => {
            spinner.failure("JMAP connection failed");
            Err(err)
        }
    }
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
        alpn: JmapClientStd::default_alpn(),
        auth,
        identity_id: None,
        drafts_mailbox_id: None,
    }
}

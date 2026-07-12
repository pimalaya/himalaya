//! Microsoft Graph API wizard (Microsoft accounts).
//!
//! The Graph API is bearer-token-only: the wizard collects the user id
//! and the token secret, typically an Ortie shell command since tokens
//! expire and need refreshing.

use anyhow::Result;
use pimalaya_cli::prompt;

use crate::{
    config::{MsgraphAuthConfig, MsgraphConfig},
    wizard::secret,
};

/// Runs the Microsoft Graph wizard, returning a ready [`MsgraphConfig`].
pub fn configure() -> Result<MsgraphConfig> {
    println!(
        "Microsoft Graph uses OAuth 2.0 tokens; issue and refresh them with an external manager such as Ortie"
    );

    let user_id = prompt::text("Microsoft Graph user id:", Some("me"))?;
    let token = secret::configure("Microsoft Graph access token", Some("ortie token show"))?;

    Ok(MsgraphConfig {
        user_id,
        tls: Default::default(),
        alpn: vec!["http/1.1".to_string()],
        auth: MsgraphAuthConfig { token },
    })
}

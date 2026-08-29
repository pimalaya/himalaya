//! # Microsoft Graph wizard
//!
//! Configures a Microsoft account against the Graph API.
//!
//! The API takes bearer tokens alone, so the prompts collect the user id
//! and the token, usually as a broker command since a token expires.

use anyhow::Result;
use pimalaya_cli::prompt;

use crate::{
    config::{MsgraphAuthConfig, MsgraphConfig},
    wizard::secret,
};

/// Runs the Microsoft Graph wizard, returning a ready [`MsgraphConfig`].
pub fn configure(account_name: &str) -> Result<MsgraphConfig> {
    eprintln!(
        "Microsoft Graph uses OAuth 2.0 tokens; issue and refresh them with an external broker such as Ortie."
    );

    let user_id = prompt::text("Microsoft Graph user id:", Some("me"))?;
    let token = secret::configure_token(
        "Microsoft Graph access token",
        &format!("{account_name}-msgraph"),
        true,
    )?;

    Ok(MsgraphConfig {
        user_id,
        tls: Default::default(),
        alpn: vec!["http/1.1".to_string()],
        auth: MsgraphAuthConfig { token },
    })
}

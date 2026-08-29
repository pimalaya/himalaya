//! # Gmail wizard
//!
//! Configures a Google account against the Gmail REST API.
//!
//! The API takes bearer tokens alone, so the prompts collect the user id
//! and the token, usually as a broker command since a token expires.

use anyhow::Result;
use pimalaya_cli::prompt;

use crate::{
    config::{GmailAuthConfig, GmailConfig},
    wizard::secret,
};

/// Runs the Gmail wizard, returning a ready [`GmailConfig`].
pub fn configure(account_name: &str) -> Result<GmailConfig> {
    eprintln!(
        "Gmail uses OAuth 2.0 tokens; issue and refresh them with an external broker such as Ortie."
    );

    let user_id = prompt::text("Gmail user id:", Some("me"))?;
    let token =
        secret::configure_token("Gmail access token", &format!("{account_name}-gmail"), true)?;

    Ok(GmailConfig {
        user_id,
        tls: Default::default(),
        alpn: vec!["http/1.1".to_string()],
        auth: GmailAuthConfig { token },
    })
}

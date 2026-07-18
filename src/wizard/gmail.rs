//! Gmail REST API wizard (Google accounts).
//!
//! The Gmail API is bearer-token-only: the wizard collects the user id
//! and the token secret, typically an Ortie shell command since tokens
//! expire and need refreshing.

use anyhow::Result;
use pimalaya_cli::prompt;

use crate::{
    config::{GmailAuthConfig, GmailConfig},
    wizard::secret,
};

/// Runs the Gmail wizard, returning a ready [`GmailConfig`].
pub fn configure(account_name: &str) -> Result<GmailConfig> {
    println!(
        "Gmail uses OAuth 2.0 tokens; issue and refresh them with an external manager such as Ortie"
    );

    let user_id = prompt::text("Gmail user id:", Some("me"))?;
    let token = secret::configure(
        "Gmail access token",
        &format!("{account_name}-gmail"),
        &secret::ortie(account_name),
    )?;

    Ok(GmailConfig {
        user_id,
        tls: Default::default(),
        alpn: vec!["http/1.1".to_string()],
        auth: GmailAuthConfig { token },
    })
}

//! # Gmail wizard
//!
//! Builds and tests the Gmail API block of a route the review settled.
//!
//! The API takes OAuth 2.0 bearer tokens alone, read from the command the
//! sign-in answered, and the mailbox is the signed-in user's, `me`.

use anyhow::Result;
use pimalaya_config::secret::{Secret, SecretResolver};

use crate::{
    account::check,
    config::{GmailAuthConfig, GmailConfig},
};

/// Reads the signed-in user's profile with `token`, answering the block
/// that connected.
pub fn connect(token: Secret) -> Result<GmailConfig> {
    let config = GmailConfig {
        user_id: String::from("me"),
        tls: Default::default(),
        proxy: None,
        alpn: vec![String::from("http/1.1")],
        auth: GmailAuthConfig { token },
    };

    check::connect_gmail(&config, &mut SecretResolver::new())?;

    Ok(config)
}

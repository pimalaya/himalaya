//! # Microsoft Graph wizard
//!
//! Builds and tests the Microsoft Graph block of a route the review
//! settled.
//!
//! The API takes OAuth 2.0 bearer tokens alone, read from the command the
//! sign-in answered, and the mailbox is the signed-in user's, `me`.

use anyhow::Result;
use pimalaya_config::secret::{Secret, SecretResolver};

use crate::{
    account::check,
    config::{MsgraphAuthConfig, MsgraphConfig},
};

/// Reads the signed-in user with `token`, answering the block that
/// connected.
pub fn connect(token: Secret) -> Result<MsgraphConfig> {
    let config = MsgraphConfig {
        user_id: String::from("me"),
        tls: Default::default(),
        proxy: None,
        alpn: vec![String::from("http/1.1")],
        auth: MsgraphAuthConfig { token },
    };

    check::connect_msgraph(&config, &mut SecretResolver::new())?;

    Ok(config)
}

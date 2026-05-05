//! Shared helper for fetching the raw RFC 5322 bytes of a single
//! message via the active account's backend.
//!
//! Used by `messages compose --reply/--forward`, `attachments list`,
//! `attachments download`, etc. Cross-backend commands always treat
//! the IMAP id as a UID — sequence-number addressing belongs to the
//! protocol-specific `imap` subcommands.

use anyhow::Result;

use crate::{
    cli::BackendArg,
    config::{AccountConfig, Config},
    email_client::build,
};

/// Fetches the raw RFC 5322 bytes of `id` from `mailbox` via the first
/// configured backend that matches `backend`. Bails when no backend
/// matches.
pub(crate) fn fetch_raw(
    config: &Config,
    account_config: &AccountConfig,
    backend: BackendArg,
    mailbox: &str,
    id: &str,
) -> Result<Vec<u8>> {
    let mut ctx = build(config.clone(), account_config.clone(), backend)?;
    Ok(ctx.client.get_message(mailbox, id)?)
}

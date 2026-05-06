//! Himalaya wrapper around [`io_jmap::client::JmapClient`] that
//! bundles the merged [`Account`] alongside the live JMAP client.
//!
//! Built up front by the dispatch layer (`crate::cli`) via
//! [`build_jmap_client`] and handed down to every JMAP-specific
//! subcommand.

use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use io_jmap::client::JmapClient as Inner;
use pimalaya_config::toml::TomlConfig;

use crate::{
    account::context::Account, cli::load_or_wizard, config::JmapConfig, jmap::session::JmapSession,
};

pub struct JmapClient {
    inner: Inner,
    pub account: Account,
    /// The original JMAP config block, kept around so commands like
    /// `email import` / `email export` can spin up their own
    /// auxiliary sessions (e.g. against the upload/download URL when
    /// it lives on a different authority than the API URL).
    pub config: JmapConfig,
}

impl JmapClient {
    /// Establishes the JMAP session (TLS, `/.well-known/jmap`
    /// discovery) then wraps the resulting client alongside
    /// `account`.
    pub fn new(config: JmapConfig, account: Account) -> Result<Self> {
        let session = JmapSession::new(
            config.server.clone(),
            config.tls.clone().try_into()?,
            config.auth.clone().try_into()?,
        )?;
        let inner = Inner::from_parts(session.stream, session.http_auth, session.session);

        Ok(Self {
            inner,
            account,
            config,
        })
    }
}

impl Deref for JmapClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for JmapClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Loads the configuration, picks the active account, builds the
/// merged [`Account`] then opens the JMAP session. Bails when the
/// account has no `[jmap]` block.
pub fn build_jmap_client(
    config_paths: &[PathBuf],
    account_name: Option<&str>,
) -> Result<JmapClient> {
    let mut config = load_or_wizard(config_paths)?;
    let (name, mut ac) = config
        .take_account(account_name)?
        .ok_or_else(|| anyhow!("Cannot find account"))?;
    let jmap_config = ac
        .jmap
        .take()
        .ok_or_else(|| anyhow!("JMAP config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(ac));
    JmapClient::new(jmap_config, account)
}

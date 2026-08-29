//! # m2dir client
//!
//! The wrapper around io-m2dir's client every m2dir-specific subcommand
//! receives, built up front by the dispatch layer.

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use io_m2dir::client::M2dirClient as Inner;

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, M2dirConfig},
};

/// An m2dir client rooted at the configured store.
pub struct M2dirClient {
    inner: Inner,
}

impl M2dirClient {
    /// Builds a client rooted at the configured store path.
    pub fn new(config: M2dirConfig) -> Self {
        let inner = Inner::new(config.root.to_string_lossy().into_owned());
        Self { inner }
    }
}

impl Deref for M2dirClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for M2dirClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Opens the m2dir client of an already-resolved account, returning it
/// beside the merged [`Account`].
///
/// Bails when the account declares no `[m2dir]` block.
pub fn build_m2dir_client(
    config: Config,
    name: String,
    mut account_config: AccountConfig,
) -> Result<(Account, M2dirClient)> {
    let m2dir_config = account_config
        .m2dir
        .take()
        .ok_or_else(|| anyhow!("M2dir config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(account_config));
    Ok((account, M2dirClient::new(m2dir_config)))
}

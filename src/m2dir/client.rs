//! Himalaya wrapper around [`io_m2dir::client::M2dirClient`].

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use io_m2dir::client::M2dirClient as Inner;

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, M2dirConfig},
};

/// Live m2dir client wrapping io_m2dir with the configured store root.
pub struct M2dirClient {
    inner: Inner,
}

impl M2dirClient {
    /// Builds an [`M2dirClient`] rooted at the configured m2store
    /// path.
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

/// Opens the m2dir client for an already-resolved account: takes the
/// `[m2dir]` block out of `account_config` and builds the merged
/// [`Account`]. Bails when the account has no `[m2dir]` block. Returns
/// the client paired with the merged account so subcommands receive both
/// as sibling arguments.
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

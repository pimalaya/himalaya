// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Himalaya wrapper around [`io_maildir::client::MaildirClient`]
//! that bundles the merged [`Account`] alongside the maildir client.
//!
//! Built up front by the dispatch layer (`crate::cli`) via
//! [`build_maildir_client`] and handed down to every maildir-specific
//! subcommand.

use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use io_maildir::client::MaildirClient as Inner;
use pimalaya_config::toml::TomlConfig;

use crate::{account::context::Account, cli::load_or_wizard, config::MaildirConfig};

pub struct MaildirClient {
    inner: Inner,
    pub account: Account,
    /// Filesystem root of the configured maildir. Kept on the wrapper
    /// so commands can join sub-paths (per-mailbox) without needing
    /// the original [`MaildirConfig`].
    pub root: PathBuf,
}

impl MaildirClient {
    /// Builds a [`MaildirClient`] rooted at the configured maildir
    /// path.
    pub fn new(config: MaildirConfig, account: Account) -> Self {
        let root = config.root.clone();
        let inner = Inner::new(config.root);
        Self {
            inner,
            account,
            root,
        }
    }
}

impl Deref for MaildirClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MaildirClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Loads the configuration, picks the active account, builds the
/// merged [`Account`] then opens the maildir client. Bails when the
/// account has no `[maildir]` block.
pub fn build_maildir_client(
    config_paths: &[PathBuf],
    account_name: Option<&str>,
) -> Result<MaildirClient> {
    let mut config = load_or_wizard(config_paths)?;
    let (name, mut ac) = config
        .take_account(account_name)?
        .ok_or_else(|| anyhow!("Cannot find account"))?;
    let maildir_config = ac
        .maildir
        .take()
        .ok_or_else(|| anyhow!("Maildir config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(ac));
    Ok(MaildirClient::new(maildir_config, account))
}

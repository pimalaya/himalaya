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

//! Himalaya wrapper around [`io_m2dir::client::M2dirClient`].

use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use io_m2dir::client::M2dirClient as Inner;
use pimalaya_config::toml::TomlConfig;

use crate::{account::context::Account, cli::load_or_wizard, config::M2dirConfig};

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

/// Loads the configuration, picks the active account, builds the
/// merged [`Account`] then opens the m2dir client. Bails when the
/// account has no `[m2dir]` block. Returns the client paired with
/// the merged account so subcommands receive both as sibling
/// arguments.
pub fn build_m2dir_client(
    config_paths: &[PathBuf],
    account_name: Option<&str>,
) -> Result<(Account, M2dirClient)> {
    let mut config = load_or_wizard(config_paths)?;
    let (name, mut ac) = config
        .take_account(account_name)?
        .ok_or_else(|| anyhow!("Cannot find account"))?;
    let m2dir_config = ac
        .m2dir
        .take()
        .ok_or_else(|| anyhow!("m2dir config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(ac));
    Ok((account, M2dirClient::new(m2dir_config)))
}

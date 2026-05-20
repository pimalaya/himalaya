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

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;
use pimalaya_config::toml::TomlConfig;

use crate::{config::Config, wizard};

/// Edit (or create) the given account through the wizard.
///
/// Loads the configuration if any, then runs the IMAP and SMTP
/// wizards with the account's current values as defaults. Provider
/// discovery is skipped: the wizard prompts you for each field with
/// what you previously had. Creates a new account if `name` is not
/// known.
#[derive(Debug, Parser)]
pub struct AccountConfigureCommand {
    /// Name of the account to edit. A new entry is created if no
    /// account with this name exists in the configuration.
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl AccountConfigureCommand {
    pub fn execute(self, _printer: &mut impl Printer, config_paths: &[PathBuf]) -> Result<()> {
        let target = Config::target_path(config_paths)?;
        let config = Config::from_paths_or_default(config_paths)?.unwrap_or_default();

        wizard::edit::edit_account(&target, config, &self.name)?;

        Ok(())
    }
}

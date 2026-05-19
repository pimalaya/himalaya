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

use std::path::PathBuf;

use clap::Parser;
use color_eyre::Result;
use pimalaya_tui::{himalaya::wizard, terminal::config::TomlConfig as _};
use tracing::info;

use crate::{account::arg::name::AccountNameArg, config::TomlConfig};

/// Configure an account.
///
/// This command is mostly used to define or reset passwords managed
/// by your global keyring. If you do not use the keyring system, you
/// can skip this command.
#[derive(Debug, Parser)]
pub struct AccountConfigureCommand {
    #[command(flatten)]
    pub account: AccountNameArg,

    /// Reset keyring passwords.
    ///
    /// This argument will force passwords to be prompted again, then
    /// saved to your global keyring.
    #[arg(long, short)]
    pub reset: bool,
}

impl AccountConfigureCommand {
    pub async fn execute(
        self,
        mut config: TomlConfig,
        config_path: Option<&PathBuf>,
    ) -> Result<()> {
        info!("executing account configure command");

        let path = match config_path {
            Some(path) => path.clone(),
            None => TomlConfig::default_path()?,
        };

        let account_name = Some(self.account.name.as_str());

        let account_config = config
            .accounts
            .remove(&self.account.name)
            .unwrap_or_default();

        wizard::edit(path, config, account_name, account_config).await?;

        Ok(())
    }
}

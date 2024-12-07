use std::path::PathBuf;

use clap::Parser;
use color_eyre::Result;

use crate::{account::arg::name::AccountNameArg, config::TomlConfig};

/// Configure the given account.
///
/// This command allows you to configure an existing account or to
/// create a new one, using the wizard. The `wizard` cargo feature is
/// required.
#[derive(Debug, Parser)]
pub struct AccountConfigureCommand {
    #[command(flatten)]
    pub account: AccountNameArg,
}

impl AccountConfigureCommand {
    #[cfg(feature = "wizard")]
    pub async fn execute(
        self,
        mut config: TomlConfig,
        config_path: Option<&PathBuf>,
    ) -> Result<()> {
        use pimalaya_tui::{himalaya::wizard, terminal::config::TomlConfig as _};
        use tracing::info;

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

    #[cfg(not(feature = "wizard"))]
    pub async fn execute(self, _: TomlConfig, _: Option<&PathBuf>) -> Result<()> {
        color_eyre::eyre::bail!("This command requires the `wizard` cargo feature to work");
    }
}

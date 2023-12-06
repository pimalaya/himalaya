use anyhow::Result;
use clap::Parser;
use log::info;

use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, cache::arg::disable::DisableCacheFlag,
    config::TomlConfig, folder::arg::name::FolderNameArg, printer::Printer,
};

/// Create a new folder
#[derive(Debug, Parser)]
pub struct FolderCreateCommand {
    #[command(flatten)]
    pub folder: FolderNameArg,

    #[command(flatten)]
    pub account: AccountNameFlag,

    #[command(flatten)]
    pub cache: DisableCacheFlag,
}

impl FolderCreateCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing folder create command");

        let folder = &self.folder.name;

        let some_account_name = self.account.name.as_ref().map(String::as_str);
        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(some_account_name, self.cache.disable)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), false).await?;

        backend.add_folder(&folder).await?;
        printer.print(format!("Folder {folder} successfully created!"))?;

        Ok(())
    }
}

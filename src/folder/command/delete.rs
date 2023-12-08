use anyhow::Result;
use clap::Parser;
use dialoguer::Confirm;
use log::info;
use std::process;

use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, cache::arg::disable::CacheDisableFlag,
    config::TomlConfig, folder::arg::name::FolderNameArg, printer::Printer,
};

/// Delete a folder.
///
/// All emails from the given folder are definitely deleted. The
/// folder is also deleted after execution of the command.
#[derive(Debug, Parser)]
pub struct FolderDeleteCommand {
    #[command(flatten)]
    pub folder: FolderNameArg,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl FolderDeleteCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing folder delete command");

        let folder = &self.folder.name;

        let confirm_msg = format!("Do you really want to delete the folder {folder}? All emails will be definitely deleted.");
        let confirm = Confirm::new()
            .with_prompt(confirm_msg)
            .default(false)
            .report(false)
            .interact_opt()?;
        if let Some(false) | None = confirm {
            process::exit(0);
        };

        let some_account_name = self.account.name.as_ref().map(String::as_str);
        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(some_account_name, self.cache.disable)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), false).await?;

        backend.delete_folder(&folder).await?;
        printer.print(format!("Folder {folder} successfully deleted!"))?;

        Ok(())
    }
}

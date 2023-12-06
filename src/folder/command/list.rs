use anyhow::Result;
use clap::Parser;
use log::info;

use crate::{
    account::arg::name::AccountNameFlag,
    backend::Backend,
    cache::arg::disable::DisableCacheFlag,
    config::TomlConfig,
    folder::Folders,
    printer::{PrintTableOpts, Printer},
    ui::arg::max_width::MaxTableWidthFlag,
};

/// List all folders
#[derive(Debug, Parser)]
pub struct FolderListCommand {
    #[command(flatten)]
    pub table: MaxTableWidthFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,

    #[command(flatten)]
    pub cache: DisableCacheFlag,
}

impl FolderListCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing folder list command");

        let some_account_name = self.account.name.as_ref().map(String::as_str);
        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(some_account_name, self.cache.disable)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), false).await?;

        let folders: Folders = backend.list_folders().await?.into();

        printer.print_table(
            Box::new(folders),
            PrintTableOpts {
                format: &account_config.email_reading_format,
                max_width: self.table.max_width,
            },
        )?;

        Ok(())
    }
}

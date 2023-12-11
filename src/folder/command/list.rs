use anyhow::Result;
use clap::Parser;
use log::info;

use crate::{
    account::arg::name::AccountNameFlag,
    backend::Backend,
    cache::arg::disable::CacheDisableFlag,
    config::TomlConfig,
    folder::Folders,
    printer::{PrintTableOpts, Printer},
    ui::arg::max_width::TableMaxWidthFlag,
};

/// List all folders.
///
/// This command allows you to list all exsting folders.
#[derive(Debug, Parser)]
pub struct FolderListCommand {
    #[command(flatten)]
    pub table: TableMaxWidthFlag,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
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
                format: &account_config.get_message_read_format(),
                max_width: self.table.max_width,
            },
        )?;

        Ok(())
    }
}

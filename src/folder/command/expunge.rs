use anyhow::Result;
use clap::Parser;
use log::info;

use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, cache::arg::disable::CacheDisableFlag,
    config::TomlConfig, folder::arg::name::FolderNameArg, printer::Printer,
};

/// Expunge a folder.
///
/// The concept of expunging is similar to the IMAP one: it definitely
/// deletes emails from the given folder that contain the "deleted"
/// flag.
#[derive(Debug, Parser)]
pub struct FolderExpungeCommand {
    #[command(flatten)]
    pub folder: FolderNameArg,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl FolderExpungeCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing folder expunge command");

        let folder = &self.folder.name;

        let some_account_name = self.account.name.as_ref().map(String::as_str);
        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(some_account_name, self.cache.disable)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), false).await?;

        backend.expunge_folder(&folder).await?;
        printer.print(format!("Folder {folder} successfully expunged!"))?;

        Ok(())
    }
}

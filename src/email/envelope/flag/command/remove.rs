use anyhow::Result;
use clap::Parser;
use log::info;

use crate::{
    account::arg::name::AccountNameFlag,
    backend::Backend,
    cache::arg::disable::DisableCacheFlag,
    config::TomlConfig,
    flag::arg::ids_and_flags::{to_tuple, IdsAndFlagsArgs},
    folder::arg::name::FolderNameArg,
    printer::Printer,
};

/// Remove flag(s) from an envelope
#[derive(Debug, Parser)]
pub struct FlagRemoveCommand {
    #[command(flatten)]
    pub folder: FolderNameArg,

    #[command(flatten)]
    pub args: IdsAndFlagsArgs,

    #[command(flatten)]
    pub account: AccountNameFlag,

    #[command(flatten)]
    pub cache: DisableCacheFlag,
}

impl FlagRemoveCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing flag remove command");

        let folder = &self.folder.name;
        let account = self.account.name.as_ref().map(String::as_str);
        let cache = self.cache.disable;

        let (toml_account_config, account_config) =
            config.clone().into_account_configs(account, cache)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), false).await?;

        let (ids, flags) = to_tuple(&self.args.ids_and_flags);
        backend.remove_flags(folder, &ids, &flags).await?;

        printer.print(format!("Flag(s) {flags} successfully removed!"))
    }
}

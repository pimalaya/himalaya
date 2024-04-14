use clap::Parser;
use color_eyre::Result;
use email::{backend::feature::BackendFeatureSource, folder::list::ListFolders};
use tracing::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag,
    backend::Backend,
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

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl FolderListCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing list folders command");

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let list_folders_kind = toml_account_config.list_folders_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config.clone(),
            list_folders_kind,
            |builder| builder.set_list_folders(BackendFeatureSource::Context),
        )
        .await?;

        let folders: Folders = backend.list_folders().await?.into();

        printer.print_table(
            Box::new(folders),
            PrintTableOpts {
                format: &account_config.get_message_read_format(),
                max_width: self.table.max_width,
            },
        )
    }
}

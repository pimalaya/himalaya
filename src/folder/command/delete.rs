use clap::Parser;
use color_eyre::Result;
use email::{backend::feature::BackendFeatureSource, folder::delete::DeleteFolder};
use inquire::Confirm;
use std::process;
use tracing::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, config::TomlConfig,
    folder::arg::name::FolderNameArg, printer::Printer,
};

/// Delete a folder.
///
/// All emails from the given folder are definitely deleted. The
/// folder is also deleted after execution of the command.
#[derive(Debug, Parser)]
pub struct FolderDeleteCommand {
    #[command(flatten)]
    pub folder: FolderNameArg,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl FolderDeleteCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing delete folder command");

        let folder = &self.folder.name;

        let confirm = Confirm::new(&format!("Do you really want to delete the folder {folder}? All emails will be definitely deleted."))
        .with_default(false).prompt_skippable()?;

        if let Some(false) | None = confirm {
            process::exit(0);
        };

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let delete_folder_kind = toml_account_config.delete_folder_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config,
            delete_folder_kind,
            |builder| builder.set_delete_folder(BackendFeatureSource::Context),
        )
        .await?;

        backend.delete_folder(folder).await?;

        printer.log(format!("Folder {folder} successfully deleted!"))
    }
}

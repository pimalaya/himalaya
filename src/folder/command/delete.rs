use std::process;

use clap::Parser;
use color_eyre::Result;
use email::{backend::feature::BackendFeatureSource, folder::delete::DeleteFolder};
use pimalaya_tui::prompt;
use tracing::info;

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

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl FolderDeleteCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing delete folder command");

        let folder = &self.folder.name;

        let confirm = format!("Do you really want to delete the folder {folder}? All emails will be definitely deleted.");

        if !prompt::bool(confirm, false)? {
            process::exit(0);
        };

        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(self.account.name.as_deref())?;

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

use clap::Parser;
use color_eyre::Result;
use email::{backend::feature::BackendFeatureSource, folder::add::AddFolder};
use tracing::info;

use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, config::Config,
    folder::arg::name::FolderNameArg, printer::Printer,
};

/// Create a new folder.
///
/// This command allows you to create a new folder using the given
/// name.
#[derive(Debug, Parser)]
pub struct AddFolderCommand {
    #[command(flatten)]
    pub folder: FolderNameArg,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl AddFolderCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &Config) -> Result<()> {
        info!("executing create folder command");

        let folder = &self.folder.name;
        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(self.account.name.as_deref())?;

        let add_folder_kind = toml_account_config.add_folder_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config,
            add_folder_kind,
            |builder| builder.set_add_folder(BackendFeatureSource::Context),
        )
        .await?;

        backend.add_folder(folder).await?;

        printer.log(format!("Folder {folder} successfully created!"))
    }
}

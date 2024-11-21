use std::sync::Arc;

use clap::Parser;
use color_eyre::Result;
use email::{
    config::Config,
    {backend::feature::BackendFeatureSource, folder::add::AddFolder},
};
use pimalaya_tui::{
    himalaya::backend::BackendBuilder,
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use tracing::info;

use crate::{
    account::arg::name::AccountNameFlag, config::TomlConfig, folder::arg::name::FolderNameArg,
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
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing create folder command");

        let folder = &self.folder.name;

        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(self.account.name.as_deref(), |c: &Config, name| {
                c.account(name).ok()
            })?;

        let backend = BackendBuilder::new(
            Arc::new(toml_account_config),
            Arc::new(account_config),
            |builder| {
                builder
                    .without_features()
                    .with_add_folder(BackendFeatureSource::Context)
            },
        )
        .without_sending_backend()
        .build()
        .await?;

        backend.add_folder(folder).await?;

        printer.out(format!("Folder {folder} successfully created!\n"))
    }
}

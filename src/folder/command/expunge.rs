use std::sync::Arc;

use clap::Parser;
use color_eyre::Result;
use email::{
    backend::feature::BackendFeatureSource, config::Config, folder::expunge::ExpungeFolder,
};
use pimalaya_tui::{
    himalaya::backend::BackendBuilder,
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use tracing::info;

use crate::{
    account::arg::name::AccountNameFlag, config::TomlConfig, folder::arg::name::FolderNameArg,
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
    pub account: AccountNameFlag,
}

impl FolderExpungeCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing expunge folder command");

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
                    .with_expunge_folder(BackendFeatureSource::Context)
            },
        )
        .without_sending_backend()
        .build()
        .await?;

        backend.expunge_folder(folder).await?;

        printer.out(format!("Folder {folder} successfully expunged!\n"))
    }
}

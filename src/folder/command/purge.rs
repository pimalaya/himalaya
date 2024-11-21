use std::{process, sync::Arc};

use clap::Parser;
use color_eyre::Result;
use email::{backend::feature::BackendFeatureSource, config::Config, folder::purge::PurgeFolder};
use pimalaya_tui::{
    himalaya::backend::BackendBuilder,
    terminal::{cli::printer::Printer, config::TomlConfig as _, prompt},
};
use tracing::info;

use crate::{
    account::arg::name::AccountNameFlag, config::TomlConfig, folder::arg::name::FolderNameArg,
};

/// Purge a folder.
///
/// All emails from the given folder are definitely deleted. The
/// purged folder will remain empty after execution of the command.
#[derive(Debug, Parser)]
pub struct FolderPurgeCommand {
    #[command(flatten)]
    pub folder: FolderNameArg,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl FolderPurgeCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing purge folder command");

        let folder = &self.folder.name;

        let confirm = format!("Do you really want to purge the folder {folder}");
        let confirm = format!("{confirm}? All emails will be definitely deleted.");

        if !prompt::bool(confirm, false)? {
            process::exit(0);
        };

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
                    .with_purge_folder(BackendFeatureSource::Context)
            },
        )
        .without_sending_backend()
        .build()
        .await?;

        backend.purge_folder(folder).await?;

        printer.out(format!("Folder {folder} successfully purged!\n"))
    }
}

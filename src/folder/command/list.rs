use std::sync::Arc;

use clap::Parser;
use color_eyre::Result;
use email::{
    config::Config,
    {backend::feature::BackendFeatureSource, folder::list::ListFolders},
};
use pimalaya_tui::{
    himalaya::{
        backend::BackendBuilder,
        config::{Folders, FoldersTable},
    },
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use tracing::info;

use crate::{account::arg::name::AccountNameFlag, config::TomlConfig};

/// List all folders.
///
/// This command allows you to list all exsting folders.
#[derive(Debug, Parser)]
pub struct FolderListCommand {
    #[command(flatten)]
    pub account: AccountNameFlag,

    /// The maximum width the table should not exceed.
    ///
    /// This argument will force the table not to exceed the given
    /// width in pixels. Columns may shrink with ellipsis in order to
    /// fit the width.
    #[arg(long, short = 'w', name = "table_max_width", value_name = "PIXELS")]
    pub table_max_width: Option<u16>,
}

impl FolderListCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing list folders command");

        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(self.account.name.as_deref(), |c: &Config, name| {
                c.account(name).ok()
            })?;

        let toml_account_config = Arc::new(toml_account_config);

        let backend = BackendBuilder::new(
            toml_account_config.clone(),
            Arc::new(account_config),
            |builder| {
                builder
                    .without_features()
                    .with_list_folders(BackendFeatureSource::Context)
            },
        )
        .without_sending_backend()
        .build()
        .await?;

        let folders = Folders::from(backend.list_folders().await?);
        let table = FoldersTable::from(folders)
            .with_some_width(self.table_max_width)
            .with_some_preset(toml_account_config.folder_list_table_preset())
            .with_some_name_color(toml_account_config.folder_list_table_name_color())
            .with_some_desc_color(toml_account_config.folder_list_table_desc_color());

        printer.out(table)?;
        Ok(())
    }
}

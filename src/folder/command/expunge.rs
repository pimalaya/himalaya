use clap::Parser;
use color_eyre::Result;
use email::{backend::feature::BackendFeatureSource, folder::expunge::ExpungeFolder};
use tracing::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, config::TomlConfig,
    folder::arg::name::FolderNameArg, printer::Printer,
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

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl FolderExpungeCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing expunge folder command");

        let folder = &self.folder.name;
        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let expunge_folder_kind = toml_account_config.expunge_folder_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config,
            expunge_folder_kind,
            |builder| builder.set_expunge_folder(BackendFeatureSource::Context),
        )
        .await?;

        backend.expunge_folder(folder).await?;

        printer.print(format!("Folder {folder} successfully expunged!"))
    }
}
